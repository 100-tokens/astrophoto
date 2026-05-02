pub mod deletion;
pub mod export;
pub mod get;
pub mod preferences;
pub mod profile;
pub mod queries;
pub mod sessions;

use crate::api_types::User;
use queries::UserRow;

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id.to_string(),
            email: r.email,
            display_name: r.display_name,
            created_at: r.created_at.to_rfc3339(),
            following_ids: vec![],
            pending_deletion_at: None,
        }
    }
}
