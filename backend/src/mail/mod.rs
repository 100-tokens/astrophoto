//! Mailer wrapping `lettre` over SMTP. Dev points at MailHog (no auth);
//! prod points at AWS SES SMTP. Bodies are plain text only — keep the
//! design's "EMAIL PREVIEW · PLAIN TEXT" promise honest.

use std::sync::{Arc, Mutex};

use lettre::{
    AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{AsyncSmtpTransport, authentication::Credentials},
};

use crate::AppError;

pub mod templates;

#[derive(Clone, Debug)]
pub struct SentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone)]
pub enum Mailer {
    Smtp {
        transport: Arc<AsyncSmtpTransport<Tokio1Executor>>,
        from: Mailbox,
    },
    Memory {
        from: Mailbox,
        outbox: Arc<Mutex<Vec<SentMail>>>,
    },
}

impl Mailer {
    pub fn from_env(cfg: &crate::config::Config) -> Result<Self, AppError> {
        let from: Mailbox = cfg.mail_from.parse().map_err(|e| {
            AppError::internal(format!("invalid MAIL_FROM '{}': {e}", cfg.mail_from))
        })?;

        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
                .port(cfg.smtp_port);
        if !cfg.smtp_user.is_empty() {
            builder = builder
                .credentials(Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone()));
        }
        Ok(Mailer::Smtp {
            transport: Arc::new(builder.build()),
            from,
        })
    }

    pub fn for_test() -> (Self, Arc<Mutex<Vec<SentMail>>>) {
        let outbox = Arc::new(Mutex::new(Vec::new()));
        #[allow(clippy::expect_used)]
        let from: Mailbox = "test <test@astrophoto.local>"
            .parse()
            .expect("valid mailbox literal");
        (
            Mailer::Memory {
                from,
                outbox: outbox.clone(),
            },
            outbox,
        )
    }

    pub async fn send_plain(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let (from, send_smtp) = match self {
            Mailer::Smtp { transport, from } => (from.clone(), Some(transport.clone())),
            Mailer::Memory { from, outbox } => {
                outbox
                    .lock()
                    .map_err(|_| AppError::internal("mail outbox lock poisoned"))?
                    .push(SentMail {
                        to: to.to_string(),
                        subject: subject.to_string(),
                        body: body.to_string(),
                    });
                (from.clone(), None)
            }
        };

        if let Some(transport) = send_smtp {
            let to_mailbox: Mailbox = to.parse().map_err(|e| {
                AppError::bad_request(format!("invalid recipient '{to}': {e}"))
            })?;
            let msg = Message::builder()
                .from(from)
                .to(to_mailbox)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
                .map_err(|e| AppError::internal(format!("mail build failed: {e}")))?;
            transport
                .send(msg)
                .await
                .map_err(|e| AppError::internal(format!("smtp send failed: {e}")))?;
        }
        Ok(())
    }
}
