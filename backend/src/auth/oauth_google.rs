use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
    reqwest::async_http_client,
};
use serde::Deserialize;

use crate::AppError;
use crate::auth::{oauth::OauthState, session};
use crate::http::AppState;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

fn client(state: &AppState) -> Result<BasicClient, AppError> {
    let cfg = &state.config;
    if cfg.oauth_google_client_id.is_empty() {
        return Err(AppError::Internal("Google OAuth not configured".into()));
    }
    let client = BasicClient::new(
        ClientId::new(cfg.oauth_google_client_id.clone()),
        Some(ClientSecret::new(cfg.oauth_google_client_secret.clone())),
        AuthUrl::new(AUTH_URL.into()).map_err(|e| AppError::Internal(e.to_string()))?,
        Some(TokenUrl::new(TOKEN_URL.into()).map_err(|e| AppError::Internal(e.to_string()))?),
    )
    .set_redirect_uri(
        RedirectUrl::new(cfg.oauth_google_redirect_url.clone())
            .map_err(|e| AppError::Internal(e.to_string()))?,
    );
    Ok(client)
}

#[allow(clippy::unwrap_used)]
pub async fn start(State(state): State<AppState>) -> Result<Response, AppError> {
    let client = client(&state)?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state_payload = OauthState {
        csrf_token: csrf_token.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
    };
    let cookie_body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&state_payload).unwrap());
    let secure_attr = if state.config.session_secure {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "{name}={body}; HttpOnly{secure}; SameSite=Lax; Path=/; Max-Age=600",
        name = crate::auth::oauth::oauth_cookie_name(state.config.session_secure),
        body = cookie_body,
        secure = secure_attr,
    );

    let mut resp = Redirect::temporary(auth_url.as_str()).into_response();
    resp.headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    Ok(resp)
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct Userinfo {
    sub: String,
    email: String,
    // Google asserts this only for addresses it has actually verified
    // (absent => false via serde default). Account linking and creation
    // below MUST gate on it — an unverified email claim matching an
    // existing local account would otherwise be an account takeover.
    #[serde(default)]
    email_verified: bool,
    name: Option<String>,
}

#[allow(clippy::unwrap_used)]
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Result<Response, AppError> {
    if let Some(err) = q.error {
        return Err(AppError::Validation(format!("oauth error: {err}")));
    }
    let code = q
        .code
        .ok_or_else(|| AppError::Validation("missing code".into()))?;
    let returned_state = q
        .state
        .ok_or_else(|| AppError::Validation("missing state".into()))?;

    // Read state cookie (accept both __Host- prefixed and unprefixed forms).
    let cookie_value = headers
        .get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            s.split(';').map(str::trim).find_map(|kv| {
                crate::auth::oauth::OAUTH_COOKIE_NAMES
                    .iter()
                    .find_map(|name| kv.strip_prefix(&format!("{name}=")))
            })
        })
        .ok_or_else(|| AppError::Validation("missing state cookie".into()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(cookie_value)
        .map_err(|_| AppError::Validation("bad state cookie".into()))?;
    let saved: OauthState = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Validation("bad state payload".into()))?;
    if saved.csrf_token != returned_state {
        return Err(AppError::Validation("state mismatch".into()));
    }

    // Exchange code for token
    let client = client(&state)?;
    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(saved.pkce_verifier))
        .request_async(async_http_client)
        .await
        .map_err(|e| AppError::Internal(format!("oauth exchange: {e}")))?;

    // Fetch userinfo
    let info: Userinfo = reqwest::Client::new()
        .get(USERINFO_URL)
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("userinfo: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("userinfo parse: {e}")))?;

    // Upsert: find user by oauth_identity OR by email; create if neither.
    let user_id = upsert_oauth_user(&state.pool, &info).await?;

    // Create session, redirect home
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let session_cookie = session::create(&state.pool, user_id, ua, None).await?;
    let session_header = session::cookie_header(
        &session_cookie,
        state.config.session_secure,
        session::SESSION_DAYS,
    );

    // Clear the oauth state cookie too
    let clear_oauth = format!(
        "{name}=; HttpOnly{secure}; SameSite=Lax; Path=/; Max-Age=0",
        name = crate::auth::oauth::oauth_cookie_name(state.config.session_secure),
        secure = if state.config.session_secure {
            "; Secure"
        } else {
            ""
        }
    );

    // Land back on the frontend, not the backend's root. In split-origin
    // deploys (Koyeb staging) the two are on different subdomains; without
    // this, `Redirect::to("/")` would dump the user on the backend host.
    // Falls back to relative "/" only when no CORS origin is configured —
    // matches the dev convention of running everything on one origin.
    let redirect_target = state
        .config
        .cors_origin
        .as_deref()
        .map(|o| format!("{}/", o.trim_end_matches('/')))
        .unwrap_or_else(|| "/".to_string());
    let mut resp = Redirect::to(&redirect_target).into_response();
    resp.headers_mut()
        .append("set-cookie", session_header.parse().unwrap());
    resp.headers_mut()
        .append("set-cookie", clear_oauth.parse().unwrap());
    Ok(resp)
}

async fn upsert_oauth_user(pool: &sqlx::PgPool, info: &Userinfo) -> Result<uuid::Uuid, AppError> {
    let provider = "google";
    let display = info.name.clone().unwrap_or_else(|| info.email.clone());

    // 1. Already linked?
    if let Some(row) = sqlx::query!(
        "select user_id from oauth_identities where provider = $1 and subject = $2",
        provider,
        info.sub
    )
    .fetch_optional(pool)
    .await?
    {
        return Ok(row.user_id);
    }

    // Steps 2 and 3 trust the Google-asserted email (to link to an
    // existing account, or to mint a new one pre-verified). Refuse both
    // unless Google says the address is verified. Step 1 (identity
    // already linked) is keyed on the immutable `sub` and stays usable.
    if !info.email_verified {
        return Err(AppError::Validation(
            "Google account email is not verified".into(),
        ));
    }

    // 2. User exists by email? Link them.
    if let Some(row) = sqlx::query!(r#"select id from users where email = $1"#, info.email)
        .fetch_optional(pool)
        .await?
    {
        sqlx::query!(
            "insert into oauth_identities (user_id, provider, subject) values ($1, $2, $3)",
            row.id,
            provider,
            info.sub
        )
        .execute(pool)
        .await?;
        return Ok(row.id);
    }

    // 3. Brand new account.
    // Generate a placeholder handle in the same form used by migration 0005
    // backfill. A real handle can be set later via the rename endpoint (Task 16).
    let placeholder_handle = format!("u-{}", &uuid::Uuid::new_v4().simple().to_string()[..6]);
    let row = sqlx::query!(
        r#"
        insert into users (email, display_name, handle, email_verified_at)
        values ($1, $2, $3, now())
        returning id
        "#,
        info.email,
        display,
        placeholder_handle
    )
    .fetch_one(pool)
    .await?;
    sqlx::query!(
        "insert into oauth_identities (user_id, provider, subject) values ($1, $2, $3)",
        row.id,
        provider,
        info.sub
    )
    .execute(pool)
    .await?;
    Ok(row.id)
}
