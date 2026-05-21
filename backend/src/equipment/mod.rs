pub mod autocomplete;
pub mod catalog_browse;
pub mod items_create;
pub mod items_get;
pub mod items_update;
pub mod setups;
pub mod specs;
pub mod upsert;

/// Canonical list of equipment kinds. Matches the DB check constraint set
/// by migration 0017 (`telescope, camera, mount, filter, focal_modifier,
/// guiding`). `guiding` is rare but real — exists in the staging catalog
/// (e.g. "unguided") so it must be discoverable.
///
/// Every handler validating a `kind` string against the API must re-use
/// this slice; do not redefine it inline. The slice and the SQL check
/// constraint drift independently if forgotten, which is exactly the
/// coherence bug this constant exists to prevent.
pub const VALID_KINDS: &[&str] = &[
    "telescope",
    "camera",
    "mount",
    "filter",
    "focal_modifier",
    "guiding",
];

/// Build the catalog key from a free-text display name.
///
/// Rules (intentionally narrow — punctuation is meaningful):
/// 1. Trim outer whitespace.
/// 2. Collapse runs of internal whitespace to a single space, so a
///    user-typed `"Sky-Watcher  Esprit 100 ED"` (double space) maps to
///    the same row as `"Sky-Watcher Esprit 100 ED"`.
/// 3. Lowercase.
///
/// Punctuation is preserved verbatim: `"sky-watcher"` and `"skywatcher"`
/// are deliberately distinct catalog entries because brands render their
/// names differently. Backfilling pre-existing duplicates is a separate
/// concern (the merge tool ships in another PR).
pub fn normalize_canonical(display: &str) -> String {
    display
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize_canonical;

    #[test]
    fn collapses_internal_whitespace() {
        assert_eq!(
            normalize_canonical("Sky-Watcher  Esprit 100 ED"),
            "sky-watcher esprit 100 ed"
        );
    }

    #[test]
    fn trims_outer_whitespace() {
        assert_eq!(
            normalize_canonical("  Celestron EdgeHD 8  "),
            "celestron edgehd 8"
        );
    }

    #[test]
    fn preserves_punctuation() {
        // "sky-watcher" and "skywatcher" stay distinct.
        assert_ne!(
            normalize_canonical("Sky-Watcher 200P"),
            normalize_canonical("Skywatcher 200P")
        );
    }

    #[test]
    fn lowercases() {
        assert_eq!(normalize_canonical("ZWO ASI2600MC"), "zwo asi2600mc");
    }

    #[test]
    fn empty_input_yields_empty_string() {
        assert_eq!(normalize_canonical(""), "");
        assert_eq!(normalize_canonical("   "), "");
    }

    #[test]
    fn handles_tabs_and_newlines() {
        // split_whitespace treats any unicode whitespace as a separator.
        assert_eq!(normalize_canonical("foo\tbar\nbaz"), "foo bar baz");
    }
}
