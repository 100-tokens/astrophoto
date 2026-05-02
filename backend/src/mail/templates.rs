//! Plain-text email templates. Each function returns (subject, body).
//! Bodies are short, mono-friendly, and stable copy.

pub fn password_reset(display_name: &str, link: &str, has_password: bool) -> (String, String) {
    let subject = if has_password {
        "Reset your Astrophoto password"
    } else {
        "Set a password for your Astrophoto account"
    };
    let intro = if has_password {
        "We received a request to reset your password."
    } else {
        "You don't have a password yet — set one to sign in without Google."
    };
    let body = format!(
        "Hello {display_name},\n\n\
         {intro} Open this link to continue:\n\n\
         {link}\n\n\
         This link is single-use and expires in one hour. If you didn't request \
         this, you can ignore this message — nothing changes.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject.to_string(), body)
}

pub fn email_change_request(current_email: &str, link: &str) -> (String, String) {
    let masked = mask_email(current_email);
    let subject = "Confirm your new Astrophoto email".to_string();
    let body = format!(
        "Hello,\n\n\
         A request was made to change the Astrophoto account currently registered as \
         {masked} to this address. Open the link below to confirm:\n\n\
         {link}\n\n\
         This link is single-use and expires in one hour. If you didn't request this, \
         ignore this message — nothing changes until the link is clicked.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

pub fn email_change_notification(masked_new: &str, occurred_at: &str) -> (String, String) {
    let subject = "Your Astrophoto email was changed".to_string();
    let body = format!(
        "Hello,\n\n\
         Your Astrophoto account email was changed to {masked_new} at {occurred_at}.\n\n\
         If this wasn't you, reply immediately or use \"Forgot password\" \
         on the sign-in page to recover access.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

pub fn account_deletion_scheduled(
    display_name: &str,
    when_human: &str,
    cancel_link: &str,
) -> (String, String) {
    let subject = "Your Astrophoto account is scheduled for deletion".to_string();
    let body = format!(
        "Hello {display_name},\n\n\
         Your Astrophoto account is scheduled for permanent deletion on {when_human}.\n\n\
         If you change your mind, sign in within the next 7 days and click \
         \"Cancel deletion\":\n\n\
         {cancel_link}\n\n\
         After the grace period, your photos and account data are erased and the \
         operation cannot be undone.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

pub fn account_deletion_cancelled(display_name: &str) -> (String, String) {
    let subject = "Your Astrophoto account deletion was cancelled".to_string();
    let body = format!(
        "Hello {display_name},\n\n\
         Your account deletion request has been cancelled. Welcome back.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

/// Mask `marie@example.com` → `mar***@example.com`. Used in the
/// notification-to-old-address template.
pub fn mask_email(email: &str) -> String {
    if let Some((local, domain)) = email.split_once('@') {
        let prefix: String = local.chars().take(3).collect();
        format!("{prefix}***@{domain}")
    } else {
        "***".into()
    }
}
