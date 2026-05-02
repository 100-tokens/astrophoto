//! In-memory mailer assertions. No DB, no SMTP.
#![allow(clippy::unwrap_used, clippy::panic)]

use astrophoto::mail::{Mailer, templates};

#[tokio::test]
async fn memory_mailer_records_sends() {
    let (mailer, outbox) = Mailer::for_test();
    mailer.send_plain("alice@example.com", "Hi", "Body").await.unwrap();
    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "alice@example.com");
    assert_eq!(sent[0].subject, "Hi");
    assert_eq!(sent[0].body, "Body");
}

#[test]
fn mask_email_keeps_first_three_chars() {
    assert_eq!(templates::mask_email("marie.dubois@example.fr"), "mar***@example.fr");
    assert_eq!(templates::mask_email("ab@x.io"), "ab***@x.io");
    assert_eq!(templates::mask_email("not-an-email"), "***");
}

#[test]
fn password_reset_uses_set_subject_when_no_password() {
    let (subject_set, body_set) = templates::password_reset("Marie", "https://x/r/abc", false);
    assert!(subject_set.contains("Set a password"));
    assert!(body_set.contains("https://x/r/abc"));
    let (subject_reset, _) = templates::password_reset("Marie", "https://x/r/abc", true);
    assert!(subject_reset.contains("Reset"));
}
