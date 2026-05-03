//! Bio HTML sanitisation. Server is the source of truth — any HTML
//! posted to PATCH /api/me passes through `sanitize`. The Tiptap
//! client editor (P2) is configured to match this allowlist, but
//! pasted HTML or tampered POSTs may contain anything.

use ammonia::Builder;
use std::sync::OnceLock;

pub fn sanitize(input: &str) -> String {
    let cleaner = cleaner();
    cleaner.clean(input).to_string()
}

fn cleaner() -> &'static Builder<'static> {
    static C: OnceLock<Builder<'static>> = OnceLock::new();
    C.get_or_init(|| {
        let mut b = Builder::default();
        b.tags(std::collections::HashSet::from([
            "p",
            "br",
            "strong",
            "em",
            "u",
            "h2",
            "h3",
            "h4",
            "ul",
            "ol",
            "li",
            "blockquote",
            "code",
            "a",
        ]));
        b.tag_attributes(std::collections::HashMap::from([(
            "a",
            std::collections::HashSet::from(["href"]),
        )]));
        b.url_schemes(std::collections::HashSet::from(["http", "https", "mailto"]));
        b.link_rel(Some("nofollow noopener"));
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
}
