pub mod bio;
pub mod by_handle;
pub mod cover;
pub mod deletion;
pub mod export;
pub mod featured;
pub mod get;
pub mod handle;
pub mod photos_feed;
pub mod preferences;
pub mod profile;
pub mod public_profile;
pub mod queries;
pub mod redirect_lookup;
pub mod sessions;
pub mod social_links;
pub mod stats;
pub mod storage;

use crate::api_types::{User, UserTier};
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
            tier: UserTier::Free,
        }
    }
}
