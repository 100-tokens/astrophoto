use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind: String,
    pub log: String,
    pub database_url: String,
    pub session_domain: String,
    pub session_secure: bool,
    pub public_base_url: String,

    pub s3_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_path_style: bool,

    pub cdn_base_url: String,

    /// When true, mount the backend's `/cdn/img/:id` route that performs
    /// on-the-fly resize from `display/<id>.jpg` using the `image` crate.
    /// Defaults to false (production: CloudFront fronts the bucket and
    /// this route is not needed). Set to true on dev / staging when
    /// CloudFront isn't deployed yet so the backend itself serves CDN
    /// URLs. Auto-on when `cdn_base_url` contains localhost.
    #[serde(default)]
    pub cdn_local_fallback: bool,

    /// Allowed CORS origin for browser clients (e.g. the SvelteKit app).
    /// Defaults to `http://localhost:5173` when unset (dev mode).
    pub cors_origin: Option<String>,

    #[serde(default)]
    pub oauth_google_client_id: String,
    #[serde(default)]
    pub oauth_google_client_secret: String,
    #[serde(default)]
    pub oauth_google_redirect_url: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub mail_from: String,
    /// false in dev (MailHog, plaintext), true in prod (STARTTLS for AWS SES on port 587).
    pub smtp_tls: bool,

    // ── Plate-solve service ──────────────────────────────────────────
    /// Base URL of the plate-solve HTTP service
    /// (xisf-rs-platesolve-server). Production:
    /// `https://platesolve.astrophoto.pics`. Leave unset to disable
    /// plate-solve features entirely (e.g. local dev without the
    /// service running). When `None`, the `photos::platesolve` client
    /// is not constructed; callers are expected to check for
    /// `AppState::platesolve.is_none()` before invoking it.
    #[serde(default)]
    pub platesolve_base_url: Option<String>,
    /// Bearer API key the plate-solve service requires. Required when
    /// `platesolve_base_url` is set; an empty key fails the
    /// `PlatesolveClient::new` construction at boot.
    #[serde(default)]
    pub platesolve_api_key: Option<String>,
    /// Maximum time the client waits for a single `/v1/solve` round
    /// trip, in seconds. Server-side wall-clock is 60 s by default;
    /// 90 s gives the client a buffer for body upload + network. Set
    /// higher only for very large masters or slow links.
    #[serde(default = "default_platesolve_timeout_secs")]
    pub platesolve_timeout_secs: u64,
}

fn default_platesolve_timeout_secs() -> u64 {
    90
}

impl Config {
    /// Load from env vars prefixed with `APP_`. Panics on missing required vars.
    #[allow(clippy::expect_used)]
    pub fn from_env() -> Self {
        figment::Figment::new()
            .merge(figment::providers::Env::prefixed("APP_"))
            .extract()
            .expect("invalid configuration: check APP_* environment variables")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("APP_BIND", "0.0.0.0:1234");
            jail.set_env("APP_LOG", "debug");
            jail.set_env("APP_DATABASE_URL", "postgres://x");
            jail.set_env("APP_SESSION_DOMAIN", "localhost");
            jail.set_env("APP_SESSION_SECURE", "false");
            jail.set_env("APP_PUBLIC_BASE_URL", "http://localhost:8080");
            jail.set_env("APP_S3_REGION", "us-east-1");
            jail.set_env("APP_S3_BUCKET", "b");
            jail.set_env("APP_S3_ACCESS_KEY", "a");
            jail.set_env("APP_S3_SECRET_KEY", "s");
            jail.set_env("APP_S3_PATH_STYLE", "true");
            jail.set_env("APP_CDN_BASE_URL", "http://localhost:8080/cdn");
            jail.set_env("APP_SMTP_HOST", "localhost");
            jail.set_env("APP_SMTP_PORT", "1025");
            jail.set_env("APP_SMTP_USER", "");
            jail.set_env("APP_SMTP_PASS", "");
            jail.set_env("APP_MAIL_FROM", "Astrophoto <noreply@astrophoto.local>");
            jail.set_env("APP_SMTP_TLS", "false");

            let cfg = Config::from_env();
            assert_eq!(cfg.bind, "0.0.0.0:1234");
            assert_eq!(cfg.log, "debug");
            assert!(!cfg.session_secure);
            assert!(cfg.s3_path_style);
            assert!(!cfg.smtp_tls);
            // platesolve_* default to disabled (None) when env vars unset.
            assert!(cfg.platesolve_base_url.is_none());
            assert!(cfg.platesolve_api_key.is_none());
            assert_eq!(cfg.platesolve_timeout_secs, 90);

            Ok(())
        });
    }

    #[test]
    fn platesolve_vars_round_trip() {
        figment::Jail::expect_with(|jail| {
            // Required vars from the test above, abbreviated.
            jail.set_env("APP_BIND", "0.0.0.0:1234");
            jail.set_env("APP_LOG", "debug");
            jail.set_env("APP_DATABASE_URL", "postgres://x");
            jail.set_env("APP_SESSION_DOMAIN", "localhost");
            jail.set_env("APP_SESSION_SECURE", "false");
            jail.set_env("APP_PUBLIC_BASE_URL", "http://localhost:8080");
            jail.set_env("APP_S3_REGION", "us-east-1");
            jail.set_env("APP_S3_BUCKET", "b");
            jail.set_env("APP_S3_ACCESS_KEY", "a");
            jail.set_env("APP_S3_SECRET_KEY", "s");
            jail.set_env("APP_S3_PATH_STYLE", "true");
            jail.set_env("APP_CDN_BASE_URL", "http://localhost:8080/cdn");
            jail.set_env("APP_SMTP_HOST", "localhost");
            jail.set_env("APP_SMTP_PORT", "1025");
            jail.set_env("APP_SMTP_USER", "");
            jail.set_env("APP_SMTP_PASS", "");
            jail.set_env("APP_MAIL_FROM", "Astrophoto <noreply@astrophoto.local>");
            jail.set_env("APP_SMTP_TLS", "false");

            jail.set_env(
                "APP_PLATESOLVE_BASE_URL",
                "https://platesolve.astrophoto.pics",
            );
            jail.set_env("APP_PLATESOLVE_API_KEY", "secret-key");
            jail.set_env("APP_PLATESOLVE_TIMEOUT_SECS", "120");

            let cfg = Config::from_env();
            assert_eq!(
                cfg.platesolve_base_url.as_deref(),
                Some("https://platesolve.astrophoto.pics")
            );
            assert_eq!(cfg.platesolve_api_key.as_deref(), Some("secret-key"));
            assert_eq!(cfg.platesolve_timeout_secs, 120);
            Ok(())
        });
    }
}
