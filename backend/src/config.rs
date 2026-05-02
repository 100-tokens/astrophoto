use serde::Deserialize;

fn default_smtp_host() -> String {
    "localhost".into()
}

fn default_smtp_port() -> u16 {
    1025
}

fn default_mail_from() -> String {
    "Astrophoto <noreply@astrophoto.local>".into()
}

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

    #[serde(default = "default_smtp_host")]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default)]
    pub smtp_user: String,
    #[serde(default)]
    pub smtp_pass: String,
    #[serde(default = "default_mail_from")]
    pub mail_from: String,
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

            let cfg = Config::from_env();
            assert_eq!(cfg.bind, "0.0.0.0:1234");
            assert_eq!(cfg.log, "debug");
            assert!(!cfg.session_secure);
            assert!(cfg.s3_path_style);

            Ok(())
        });
    }
}
