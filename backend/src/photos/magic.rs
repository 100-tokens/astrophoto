//! Magic-byte sniff. Backend trusts neither the client `Content-Type`
//! header nor the file extension. We range-GET the first 16 bytes from
//! S3 and check the signature against the declared mime.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniffResult {
    Jpeg,
    Png,
    Tiff,
    /// XISF (PixInsight Extensible Image Serialization Format). Recognised
    /// for the side-channel plate-solve upload only — NOT a member of the
    /// standard upload allowlist (`matches_mime`). See
    /// `crate::photos::platesolve` and `docs/platesolve-integration.md`.
    Xisf,
    Unknown,
}

/// 8-byte ASCII signature at offset 0 of every XISF v1 file
/// (`xisf-rs-core::utils::constants::XISF_SIGNATURE`).
const XISF_SIGNATURE: &[u8; 8] = b"XISF0100";

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
    if bytes.starts_with(XISF_SIGNATURE) {
        return SniffResult::Xisf;
    }
    SniffResult::Unknown
}

pub fn matches_mime(s: SniffResult, mime: &str) -> bool {
    matches!(
        (s, mime),
        (SniffResult::Jpeg, "image/jpeg")
            | (SniffResult::Png, "image/png")
            | (SniffResult::Tiff, "image/tiff")
            | (SniffResult::Xisf, "application/x-xisf")
    )
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
    fn xisf_signature() {
        assert_eq!(sniff(b"XISF0100\x00\x00\x00\x00"), SniffResult::Xisf);
    }

    #[test]
    fn xisf_wrong_version_rejected() {
        // The current spec is v1 only ("XISF0100"); higher versions are
        // not transparently accepted — when XISF v2 ships the parser
        // (and this sniff) must opt in explicitly.
        assert_eq!(sniff(b"XISF0200\x00\x00\x00\x00"), SniffResult::Unknown);
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

    #[test]
    fn xisf_matches_application_x_xisf() {
        // XISF was originally side-channel-only; once XISF became a
        // primary upload format the magic-byte sniff has to accept
        // the matching MIME for the standard finalize gate to pass.
        assert!(matches_mime(SniffResult::Xisf, "application/x-xisf"));
        // …but it still must NOT pose as a raster image MIME.
        assert!(!matches_mime(SniffResult::Xisf, "image/jpeg"));
        assert!(!matches_mime(SniffResult::Xisf, "image/png"));
    }
}
