//! CDN URL builder. Returns
//!   {base}/img/{photo_id}?w=&h=&fit=&q=&fm=
//! In prod, base is the CloudFront distribution. In dev, the backend's
//! own /cdn route serves the same URL shape locally.

use uuid::Uuid;

#[derive(Default)]
pub struct Transform {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub fit: Option<&'static str>, // "cover" | "contain"
    pub q: Option<u8>,
    pub fm: Option<&'static str>, // "auto" | "jpg" | "webp"
}

pub fn url(base: &str, photo_id: Uuid, t: &Transform) -> String {
    let mut s = format!("{base}/img/{photo_id}");
    let mut sep = '?';
    if let Some(w) = t.w {
        s.push(sep);
        s.push_str(&format!("w={w}"));
        sep = '&';
    }
    if let Some(h) = t.h {
        s.push(sep);
        s.push_str(&format!("h={h}"));
        sep = '&';
    }
    if let Some(f) = t.fit {
        s.push(sep);
        s.push_str(&format!("fit={f}"));
        sep = '&';
    }
    if let Some(q) = t.q {
        s.push(sep);
        s.push_str(&format!("q={q}"));
        sep = '&';
    }
    if let Some(fm) = t.fm {
        s.push(sep);
        s.push_str(&format!("fm={fm}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_params() {
        let id = Uuid::nil();
        assert_eq!(
            url("https://cdn.x.app", id, &Transform::default()),
            format!("https://cdn.x.app/img/{id}")
        );
    }

    #[test]
    fn full_params() {
        let id = Uuid::nil();
        let t = Transform {
            w: Some(800),
            h: Some(600),
            fit: Some("cover"),
            q: Some(85),
            fm: Some("auto"),
        };
        let got = url("https://cdn.x.app", id, &t);
        assert!(got.contains("w=800"));
        assert!(got.contains("fm=auto"));
    }
}
