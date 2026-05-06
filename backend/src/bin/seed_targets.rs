//! Parses pinned OpenNGC CSVs and UPSERTs into `targets`. Idempotent.
//! See docs/superpowers/specs/2026-05-06-celestial-objects-design.md

use anyhow::Result;

#[derive(Debug, PartialEq)]
pub struct OpenNgcRow {
    pub name: String,         // e.g. "NGC0224"
    pub messier_num: Option<u32>,
    pub ra_deg: Option<f64>,
    pub dec_deg: Option<f64>,
    pub object_type: Option<String>,
    pub constellation: Option<String>,
    pub magnitude_v: Option<f32>,
    pub major_axis_arcmin: Option<f32>,
    pub minor_axis_arcmin: Option<f32>,
    pub common_names: Vec<String>,
}

pub fn parse_csv_row(record: &csv::StringRecord, headers: &csv::StringRecord) -> Result<OpenNgcRow> {
    use anyhow::Context;

    let get = |col: &str| -> Option<&str> {
        let idx = headers.iter().position(|h| h == col)?;
        let v = record.get(idx)?.trim();
        if v.is_empty() { None } else { Some(v) }
    };

    let name = get("Name").context("missing Name column")?.to_string();
    let messier_num = get("M").and_then(|s| s.parse().ok());
    let ra_deg = get("RA").map(parse_ra_sexagesimal).transpose()?;
    let dec_deg = get("Dec").map(parse_dec_sexagesimal).transpose()?;
    let object_type = get("Type").map(|s| s.to_string());
    let constellation = get("Const").map(|s| s.to_string());
    let magnitude_v = get("V-Mag").and_then(|s| s.parse::<f32>().ok());
    let major_axis_arcmin = get("MajAx").and_then(|s| s.parse::<f32>().ok());
    let minor_axis_arcmin = get("MinAx").and_then(|s| s.parse::<f32>().ok());
    let common_names = get("Common names")
        .map(|s| s.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect())
        .unwrap_or_default();

    Ok(OpenNgcRow {
        name, messier_num, ra_deg, dec_deg, object_type, constellation,
        magnitude_v, major_axis_arcmin, minor_axis_arcmin, common_names,
    })
}

/// Parse "00:42:44.330" → degrees in [0, 360).
fn parse_ra_sexagesimal(s: &str) -> Result<f64> {
    let mut parts = s.split(':');
    let h: f64 = parts.next().ok_or_else(|| anyhow::anyhow!("RA empty"))?.parse()?;
    let m: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
    let sec: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
    Ok((h + m / 60.0 + sec / 3600.0) * 15.0)
}

/// Parse "+41:16:09.40" or "-12:34:56.7" → degrees in [-90, 90].
fn parse_dec_sexagesimal(s: &str) -> Result<f64> {
    let s = s.trim();
    let (sign, rest) = if let Some(rest) = s.strip_prefix('-') {
        (-1.0, rest)
    } else {
        (1.0, s.strip_prefix('+').unwrap_or(s))
    };
    let mut parts = rest.split(':');
    let d: f64 = parts.next().ok_or_else(|| anyhow::anyhow!("Dec empty"))?.parse()?;
    let m: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
    let sec: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
    Ok(sign * (d + m / 60.0 + sec / 3600.0))
}

fn main() -> Result<()> {
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum SlugDecision {
    Slug(String),
    Skip(SkipReason),
}

#[derive(Debug, PartialEq)]
pub enum SkipReason {
    SubcomponentSuffix, // Trailing letter on NGC/IC numeric portion (NGC5128A, NGC0292A)
    UnknownPrefix,      // Not NGC/IC, no Messier number
    Duplicate,          // OpenNGC Type='Dup' — would clobber the canonical row
}

pub fn compute_slug(row: &OpenNgcRow) -> SlugDecision {
    // Check Type='Dup' first: M102's row has M=101 Type=Dup; skipping last
    // would silently clobber the correct NGC5457 (Pinwheel) row.
    if row.object_type.as_deref() == Some("Dup") {
        return SlugDecision::Skip(SkipReason::Duplicate);
    }

    if let Some(m) = row.messier_num {
        return SlugDecision::Slug(format!("m{}", m));
    }
    let n = &row.name;
    if let Some(rest) = n.strip_prefix("NGC") {
        return parse_numeric_suffix("ngc", rest);
    }
    if let Some(rest) = n.strip_prefix("IC") {
        return parse_numeric_suffix("ic", rest);
    }
    SlugDecision::Skip(SkipReason::UnknownPrefix)
}

fn parse_numeric_suffix(prefix: &str, rest: &str) -> SlugDecision {
    if !rest.chars().all(|c| c.is_ascii_digit()) {
        return SlugDecision::Skip(SkipReason::SubcomponentSuffix);
    }
    match rest.parse::<u32>() {
        Ok(n) => SlugDecision::Slug(format!("{}-{}", prefix, n)),
        Err(_) => SlugDecision::Skip(SkipReason::UnknownPrefix),
    }
}

#[cfg(test)]
mod slug_tests {
    use super::*;

    fn row(name: &str, m: Option<u32>, object_type: Option<&str>) -> OpenNgcRow {
        OpenNgcRow {
            name: name.to_string(),
            messier_num: m,
            ra_deg: None, dec_deg: None,
            object_type: object_type.map(String::from),
            constellation: None,
            magnitude_v: None,
            major_axis_arcmin: None, minor_axis_arcmin: None,
            common_names: vec![],
        }
    }

    #[test] fn messier_slug() {
        assert_eq!(compute_slug(&row("NGC0224", Some(31), Some("G"))), SlugDecision::Slug("m31".into()));
    }
    #[test] fn ngc_slug_strips_zeros() {
        assert_eq!(compute_slug(&row("NGC0224", None, Some("G"))), SlugDecision::Slug("ngc-224".into()));
        assert_eq!(compute_slug(&row("NGC7000", None, Some("HII"))), SlugDecision::Slug("ngc-7000".into()));
    }
    #[test] fn ic_slug() {
        assert_eq!(compute_slug(&row("IC0434", None, Some("HII"))), SlugDecision::Slug("ic-434".into()));
    }
    #[test] fn skips_subcomponent() {
        assert_eq!(compute_slug(&row("NGC5128A", None, Some("G"))), SlugDecision::Skip(SkipReason::SubcomponentSuffix));
        assert_eq!(compute_slug(&row("NGC0292A", None, Some("G"))), SlugDecision::Skip(SkipReason::SubcomponentSuffix));
    }
    #[test] fn skips_unknown_prefix() {
        assert_eq!(compute_slug(&row("PGC1234", None, Some("G"))), SlugDecision::Skip(SkipReason::UnknownPrefix));
        assert_eq!(compute_slug(&row("B033", None, Some("DrkN"))), SlugDecision::Skip(SkipReason::UnknownPrefix));
    }
    #[test] fn skips_dup_type() {
        // M102 in addendum: M=101, Type=Dup. Must skip BEFORE attempting slug from M=101,
        // otherwise we'd overwrite NGC5457 (M=101, Type=G, Pinwheel Galaxy).
        assert_eq!(compute_slug(&row("M102", Some(101), Some("Dup"))), SlugDecision::Skip(SkipReason::Duplicate));
        // Sanity check: a Dup row without an M number still skips, regardless of name prefix.
        assert_eq!(compute_slug(&row("NGC1234", None, Some("Dup"))), SlugDecision::Skip(SkipReason::Duplicate));
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    fn rec(values: &[&str]) -> csv::StringRecord {
        csv::StringRecord::from(values.to_vec())
    }

    #[test]
    fn parses_galaxy_with_messier() {
        // Test fixture uses a SUBSET of OpenNGC's real columns. The parser
        // looks up columns by header name via headers.iter().position(...),
        // so the same parser handles both the 10-col fixture and the real
        // 32-col CSV. Real CSV fields beyond what we ask for are ignored.
        let headers = rec(&["Name","Type","RA","Dec","Const","MajAx","MinAx","V-Mag","M","Common names"]);
        let row = rec(&["NGC0224","G","00:42:44.330","+41:16:09.40","And","190.0","60.0","3.44","31","Andromeda Galaxy,M 31"]);
        let parsed = parse_csv_row(&row, &headers).unwrap();
        assert_eq!(parsed.name, "NGC0224");
        assert_eq!(parsed.messier_num, Some(31));
        assert!((parsed.ra_deg.unwrap() - 10.6847).abs() < 0.01);
        assert!((parsed.dec_deg.unwrap() - 41.2693).abs() < 0.01);
        assert_eq!(parsed.object_type.as_deref(), Some("G"));
        assert_eq!(parsed.constellation.as_deref(), Some("And"));
        assert_eq!(parsed.magnitude_v, Some(3.44));
        assert_eq!(parsed.common_names, vec!["Andromeda Galaxy", "M 31"]);
    }

    #[test]
    fn handles_missing_v_mag() {
        let headers = rec(&["Name","Type","RA","Dec","Const","MajAx","MinAx","V-Mag","M","Common names"]);
        let row = rec(&["NGC1234","G","02:00:00","+10:00:00","Tau","","","","",""]);
        let parsed = parse_csv_row(&row, &headers).unwrap();
        assert_eq!(parsed.magnitude_v, None);
        assert_eq!(parsed.major_axis_arcmin, None);
        assert_eq!(parsed.messier_num, None);
        assert!(parsed.common_names.is_empty());
    }
}
