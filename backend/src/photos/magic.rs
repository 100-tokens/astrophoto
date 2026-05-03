//! Magic-byte sniff. Backend trusts neither the client `Content-Type`
//! header nor the file extension. We range-GET the first 16 bytes from
//! S3 and check the signature against the declared mime.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniffResult {
    Jpeg,
    Png,
    Tiff,
    Unknown,
}

pub fn sniff(bytes: &[u8]) -> SniffResult {
    if bytes.len() < 4 {
        return SniffResult::Unknown;
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return SniffResult::Jpeg;
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return SniffResult::Png;
    }
    if bytes.starts_with(&[b'I', b'I', 0x2A, 0x00]) || bytes.starts_with(&[b'M', b'M', 0x00, 0x2A])
    {
        return SniffResult::Tiff;
    }
    SniffResult::Unknown
}

pub fn matches_mime(s: SniffResult, mime: &str) -> bool {
    match (s, mime) {
        (SniffResult::Jpeg, "image/jpeg") => true,
        (SniffResult::Png, "image/png") => true,
        (SniffResult::Tiff, "image/tiff") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_signature() {
        assert_eq!(
            sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0]),
            SniffResult::Jpeg
        );
    }

    #[test]
    fn png_signature() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n"), SniffResult::Png);
    }

    #[test]
    fn tiff_le_signature() {
        assert_eq!(sniff(b"II*\x00\x00\x00\x00\x00"), SniffResult::Tiff);
    }

    #[test]
    fn no_match_for_random() {
        assert_eq!(sniff(b"hello, world!"), SniffResult::Unknown);
    }

    #[test]
    fn matches_mime_strict() {
        assert!(matches_mime(SniffResult::Jpeg, "image/jpeg"));
        assert!(!matches_mime(SniffResult::Jpeg, "image/png"));
        assert!(!matches_mime(SniffResult::Unknown, "image/jpeg"));
    }
}
