//! Bio HTML sanitisation. Server is the source of truth — any HTML
//! posted to PATCH /api/me/profile passes through `sanitize`. The Tiptap
//! client editor (P2) is configured to emit only this same allowlist;
//! the JSON file at `backend/data/bio-allowed-tags.json` is the shared
//! source of truth between Rust and TypeScript.

use ammonia::Builder;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

pub const ALLOWED_TAGS: &[&str] = &[
    "a", "blockquote", "br", "code", "em", "h2", "h3", "h4", "li", "ol", "p",
    "strong", "u", "ul",
];

const ANCHOR_SCHEMES: &[&str] = &["http", "https", "mailto"];
const ANCHOR_REL: &str = "nofollow noopener";

pub fn sanitize(input: &str) -> String {
    cleaner().clean(input).to_string()
}

fn cleaner() -> &'static Builder<'static> {
    static C: OnceLock<Builder<'static>> = OnceLock::new();
    C.get_or_init(|| {
        let mut b = Builder::default();
        b.tags(ALLOWED_TAGS.iter().copied().collect::<HashSet<_>>());
        b.tag_attributes(HashMap::from([("a", HashSet::from(["href"]))]));
        b.url_schemes(ANCHOR_SCHEMES.iter().copied().collect::<HashSet<_>>());
        b.link_rel(Some(ANCHOR_REL));
        b
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_safe_tags() {
        let out = sanitize("<p>Hello <strong>world</strong></p>");
        assert!(out.contains("<strong>"));
    }

    #[test]
    fn strips_script() {
        let out = sanitize("<p>Hi</p><script>alert('x')</script>");
        assert!(!out.contains("script"));
    }

    #[test]
    fn strips_onclick() {
        let out = sanitize("<a href=\"https://x\" onclick=\"x()\">l</a>");
        assert!(!out.contains("onclick"));
    }

    #[test]
    fn strips_javascript_uri() {
        let out = sanitize("<a href=\"javascript:alert(1)\">l</a>");
        assert!(!out.contains("javascript:"));
    }

    #[test]
    fn forces_rel_on_links() {
        let out = sanitize("<a href=\"https://x\">l</a>");
        assert!(out.contains("rel=\"nofollow noopener\""));
    }

    #[test]
    fn strips_iframe() {
        let out = sanitize("<iframe src=\"https://x\"></iframe>");
        assert!(!out.contains("iframe"));
    }

    #[test]
    fn keeps_lists() {
        let out = sanitize("<ul><li>a</li><li>b</li></ul>");
        assert!(out.contains("<ul>") && out.contains("<li>"));
    }

    #[test]
    fn allowlist_matches_shared_json() {
        let raw = include_str!("../../data/bio-allowed-tags.json");
        let json: serde_json::Value = serde_json::from_str(raw)
            .expect("bio-allowed-tags.json must be valid JSON");
        let arr = json
            .get("tags")
            .and_then(|v| v.as_array())
            .expect("bio-allowed-tags.json must have a top-level `tags` array");
        let from_json: std::collections::BTreeSet<String> = arr
            .iter()
            .map(|v| v.as_str().expect("tag must be string").to_owned())
            .collect();
        let from_code: std::collections::BTreeSet<String> =
            ALLOWED_TAGS.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(
            from_json, from_code,
            "bio.rs ALLOWED_TAGS and bio-allowed-tags.json have drifted"
        );
    }
}
