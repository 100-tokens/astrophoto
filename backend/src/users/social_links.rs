//! Validation for the `social_links` jsonb column. The DB doesn't
//! constrain shape; this module is the only legitimate writer.

use crate::AppError;
use crate::api_types::{SocialLink, SocialPlatform};
use url::Url;

const MAX_LINKS: usize = 6;

pub fn validate_links(links: &[SocialLink]) -> Result<(), AppError> {
    if links.len() > MAX_LINKS {
        return Err(AppError::bad_request("social_links_too_many"));
    }
    let mut seen = std::collections::HashSet::new();
    for link in links {
        if !seen.insert(link.platform.clone()) {
            return Err(AppError::bad_request("social_links_duplicate_platform"));
        }
        validate_one(link)?;
    }
    Ok(())
}

fn validate_one(link: &SocialLink) -> Result<(), AppError> {
    let parsed =
        Url::parse(&link.url).map_err(|_| AppError::bad_request("social_link_url_invalid"))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::bad_request("social_link_url_scheme"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::bad_request("social_link_url_no_host"))?;
    let host = host.to_ascii_lowercase();
    let allowed: &[&str] = match link.platform {
        SocialPlatform::Twitter => &["twitter.com", "x.com"],
        SocialPlatform::Instagram => &["instagram.com", "www.instagram.com"],
        SocialPlatform::Bluesky => &["bsky.app"],
        SocialPlatform::Astrobin => &["astrobin.com", "www.astrobin.com"],
        SocialPlatform::Mastodon => &[], // any host — many instances
        SocialPlatform::Youtube => &["youtube.com", "www.youtube.com", "youtu.be"],
        SocialPlatform::Website => &[], // any host
    };
    if !allowed.is_empty()
        && !allowed
            .iter()
            .any(|d| host == *d || host.ends_with(&format!(".{d}")))
    {
        return Err(AppError::bad_request("social_link_host_mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sl(p: SocialPlatform, url: &str) -> SocialLink {
        SocialLink {
            platform: p,
            url: url.into(),
        }
    }

    #[test]
    fn accepts_canonical_twitter() {
        validate_links(&[sl(SocialPlatform::Twitter, "https://twitter.com/marie")]).unwrap();
    }

    #[test]
    fn accepts_x_com_for_twitter() {
        validate_links(&[sl(SocialPlatform::Twitter, "https://x.com/marie")]).unwrap();
    }

    #[test]
    fn rejects_wrong_host_for_platform() {
        let err = validate_links(&[sl(SocialPlatform::Twitter, "https://evil.example/marie")])
            .unwrap_err();
        assert!(format!("{err:?}").contains("social_link_host_mismatch"));
    }

    #[test]
    fn rejects_javascript_scheme() {
        let err =
            validate_links(&[sl(SocialPlatform::Website, "javascript:alert(1)")]).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("social_link_url_scheme") || msg.contains("social_link_url_invalid"));
    }

    #[test]
    fn rejects_more_than_six() {
        let many: Vec<SocialLink> = (0..7)
            .map(|_| sl(SocialPlatform::Website, "https://a.example"))
            .collect();
        let err = validate_links(&many).unwrap_err();
        assert!(format!("{err:?}").contains("social_links_too_many"));
    }

    #[test]
    fn rejects_duplicate_platform() {
        let links = vec![
            sl(SocialPlatform::Twitter, "https://twitter.com/a"),
            sl(SocialPlatform::Twitter, "https://x.com/b"),
        ];
        let err = validate_links(&links).unwrap_err();
        assert!(format!("{err:?}").contains("social_links_duplicate_platform"));
    }

    #[test]
    fn website_accepts_any_https_host() {
        validate_links(&[sl(SocialPlatform::Website, "https://marie.example.com/")]).unwrap();
    }

    #[test]
    fn mastodon_accepts_any_https_host() {
        validate_links(&[sl(
            SocialPlatform::Mastodon,
            "https://mastodon.social/@marie",
        )])
        .unwrap();
    }
}
