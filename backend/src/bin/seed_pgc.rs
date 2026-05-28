//! Parses pinned HyperLEDA / PGC CSV and UPSERTs into `targets`. Idempotent.
//! See docs/superpowers/specs/2026-05-28-celestial-identify-overlay-design.md.

use anyhow::Result;

#[derive(Debug, PartialEq)]
pub struct PgcRow {
    pub pgc: u32,
    pub objname: Option<String>,
    pub ra_deg: f64,
    pub de_deg: f64,
    pub mag_b: Option<f32>,
    pub major_axis_arcmin: f32,
    pub minor_axis_arcmin: Option<f32>,
    pub position_angle_deg: Option<f32>,
}

pub fn parse_csv_row(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
) -> Result<Option<PgcRow>> {
    use anyhow::Context;

    let get = |col: &str| -> Option<&str> {
        let idx = headers.iter().position(|h| h == col)?;
        let v = record.get(idx)?.trim();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    };

    let pgc: u32 = get("pgc")
        .context("missing pgc column")?
        .parse()
        .context("pgc not a u32")?;
    let objname = get("objname").map(|s| s.to_string());
    let ra_deg: f64 = get("ra2000").context("missing ra2000")?.parse()?;
    let de_deg: f64 = get("de2000").context("missing de2000")?.parse()?;
    let mag_b: Option<f32> = get("bt").and_then(|s| s.parse().ok());

    // logd25 stores log10(diameter in 0.1 arcmin). Missing or non-positive
    // → unusable for the overlay; drop the row (the SQL filter at extract
    // time also enforces logd25 > 0, but parse-time defence keeps tests
    // honest with hand-crafted fixtures).
    let logd25: f32 = match get("logd25").and_then(|s| s.parse().ok()) {
        Some(v) if v > 0.0 => v,
        _ => return Ok(None),
    };
    let major_axis_arcmin = (10f32.powf(logd25)) * 0.1;
    // logr25 = log10(axis ratio a/b). diameter_minor = major / 10^logr25.
    let minor_axis_arcmin = get("logr25")
        .and_then(|s| s.parse::<f32>().ok())
        .map(|logr| major_axis_arcmin / 10f32.powf(logr));
    let position_angle_deg = get("pa").and_then(|s| s.parse::<f32>().ok());

    Ok(Some(PgcRow {
        pgc,
        objname,
        ra_deg,
        de_deg,
        mag_b,
        major_axis_arcmin,
        minor_axis_arcmin,
        position_angle_deg,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::ReaderBuilder;

    const FIXTURE: &str = "\
pgc,objname,ra2000,de2000,bt,logd25,logr25,pa
2557,NGC0224,10.6847083,41.2691055,4.36,2.337,0.502,35.0
3589,IC0010,5.0791666,59.3030555,11.79,1.835,0.066,
1234567,,123.45,-67.89,18.4,0.602,,
99999,SomeName,15.0,5.0,17.0,,,
";

    #[test]
    fn parses_full_row_with_ngc_objname() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(FIXTURE.as_bytes());
        let headers = rdr.headers().unwrap().clone();
        let row = rdr.records().next().unwrap().unwrap();
        let parsed = parse_csv_row(&row, &headers).unwrap().unwrap();
        assert_eq!(parsed.pgc, 2557);
        assert_eq!(parsed.objname.as_deref(), Some("NGC0224"));
        assert!((parsed.ra_deg - 10.6847083).abs() < 1e-9);
        assert_eq!(parsed.mag_b, Some(4.36));
        // logd25=2.337 → major = 10^2.337 × 0.1 ≈ 21.73 arcmin
        assert!((parsed.major_axis_arcmin - 21.73).abs() < 0.1);
        // logr25=0.502 → minor = major / 10^0.502 ≈ 6.82 arcmin
        assert!((parsed.minor_axis_arcmin.unwrap() - 6.82).abs() < 0.1);
        assert_eq!(parsed.position_angle_deg, Some(35.0));
    }

    #[test]
    fn parses_row_with_no_position_angle_and_no_objname() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(FIXTURE.as_bytes());
        let headers = rdr.headers().unwrap().clone();
        rdr.records().next();
        rdr.records().next();
        let row = rdr.records().next().unwrap().unwrap();
        let parsed = parse_csv_row(&row, &headers).unwrap().unwrap();
        assert_eq!(parsed.pgc, 1234567);
        assert_eq!(parsed.objname, None);
        assert_eq!(parsed.position_angle_deg, None);
    }

    #[test]
    fn rejects_row_missing_logd25() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(FIXTURE.as_bytes());
        let headers = rdr.headers().unwrap().clone();
        rdr.records().next();
        rdr.records().next();
        rdr.records().next();
        let row = rdr.records().next().unwrap().unwrap();
        // Last row has empty logd25 → invalid (we filter at SQL time too,
        // but defensive parsing).
        assert!(parse_csv_row(&row, &headers).unwrap().is_none());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    todo!()
}
