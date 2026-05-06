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
