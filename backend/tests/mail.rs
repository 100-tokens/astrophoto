//! In-memory mailer assertions. No DB, no SMTP.
#![allow(clippy::unwrap_used, clippy::panic)]

use astrophoto::mail::{Mailer, templates};

#[tokio::test]
async fn memory_mailer_records_sends() {
    let (mailer, outbox) = Mailer::for_test();
    mailer
        .send_plain("alice@example.com", "Hi", "Body")
        .await
        .unwrap();
    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "alice@example.com");
    assert_eq!(sent[0].subject, "Hi");
    assert_eq!(sent[0].body, "Body");
}

#[test]
fn mask_email_keeps_first_three_chars() {
    assert_eq!(
        templates::mask_email("marie.dubois@example.fr"),
        "mar***@example.fr"
    );
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

#[tokio::test]
async fn memory_mailer_rejects_invalid_recipient() {
    let (mailer, outbox) = Mailer::for_test();
    let err = mailer.send_plain("not-an-email", "Hi", "Body").await.unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.to_lowercase().contains("invalid"), "expected invalid-recipient error, got: {msg}");
    assert!(outbox.lock().unwrap().is_empty(), "no mail must be queued for invalid recipient");
}

#[test]
fn email_change_request_masks_current_email() {
    let (_subject, body) = templates::email_change_request("marie.dubois@example.fr", "https://x/c/abc");
    assert!(body.contains("mar***@example.fr"), "current email must be masked, body was: {body}");
    assert!(!body.contains("marie.dubois"), "full local-part must not appear");
}
