pub mod queries;

use crate::api_types::User;
use queries::UserRow;

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id.to_string(),
            email: r.email,
            display_name: r.display_name,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}
