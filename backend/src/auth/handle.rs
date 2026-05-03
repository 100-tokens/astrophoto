//! Handle validation: regex + reserved-list check.

use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum HandleError {
    #[error("handle must be 3-30 chars of [a-z0-9_-]")]
    Format,
    #[error("handle is reserved")]
    Reserved,
}

fn reserved() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        include_str!("../../data/reserved_handles.txt")
            .lines()
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

fn is_valid_format(h: &str) -> bool {
    let len = h.chars().count();
    if !(3..=30).contains(&len) {
        return false;
    }
    h.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

pub fn validate(handle: &str) -> Result<(), HandleError> {
    let h = handle.trim();
    if !is_valid_format(h) {
        return Err(HandleError::Format);
    }
    if reserved().contains(&h.to_ascii_lowercase()) {
        return Err(HandleError::Reserved);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_handle() {
        assert_eq!(validate("marie"), Ok(()));
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(validate("ab"), Err(HandleError::Format));
    }

    #[test]
    fn rejects_uppercase() {
        assert_eq!(validate("Marie"), Err(HandleError::Format));
    }

    #[test]
    fn rejects_reserved() {
        assert_eq!(validate("admin"), Err(HandleError::Reserved));
    }

    #[test]
    fn accepts_underscore_and_hyphen() {
        assert_eq!(validate("a_b-c"), Ok(()));
    }
}
