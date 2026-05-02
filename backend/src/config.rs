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

            Ok(())
        });
    }
}
