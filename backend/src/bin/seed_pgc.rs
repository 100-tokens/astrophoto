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
        if v.is_empty() { None } else { Some(v) }
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
    fn dedup_extracts_ngc_ref() {
        assert_eq!(extract_existing_slug_ref("NGC0224"), Some("ngc-224".into()));
        assert_eq!(extract_existing_slug_ref("NGC 224"), Some("ngc-224".into()));
        assert_eq!(extract_existing_slug_ref("NGC0224A"), None); // subcomponent
        assert_eq!(extract_existing_slug_ref("IC0010"), Some("ic-10".into()));
        assert_eq!(extract_existing_slug_ref("IC 1396"), Some("ic-1396".into()));
        assert_eq!(extract_existing_slug_ref("Andromeda Galaxy"), None);
        assert_eq!(extract_existing_slug_ref(""), None);
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

/// If `objname` looks like an existing NGC/IC catalog reference (e.g. "NGC0224"
/// or "IC 1396"), return the corresponding `targets.slug` ("ngc-224", "ic-1396")
/// so the seed binary can skip the PGC row in favour of the canonical entry.
/// Returns `None` for free-form names, blanks, and subcomponent refs ("NGC0224A").
fn extract_existing_slug_ref(objname: &str) -> Option<String> {
    let trimmed = objname.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Accept "NGC0224", "NGC 224", "IC0010", "IC 10". Reject subcomponents
    // like "NGC0224A" / "NGC0224-1" — our slug scheme has no equivalent.
    for prefix in ["NGC", "IC"] {
        let Some(rest) = trimmed.strip_prefix(prefix) else {
            continue;
        };
        let rest = rest.trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            continue;
        }
        let after_digits = &rest[digits.len()..];
        // Must be followed by end-of-string or whitespace; reject suffixes.
        if !after_digits.is_empty() && !after_digits.starts_with(char::is_whitespace) {
            return None;
        }
        if let Ok(n) = digits.parse::<u32>() {
            return Some(format!("{}-{}", prefix.to_ascii_lowercase(), n));
        }
    }
    None
}

async fn upsert_pgc_row(pool: &sqlx::PgPool, row: &PgcRow) -> Result<UpsertOutcome> {
    let slug = format!("pgc-{}", row.pgc);

    // Dedup: if the objname references an existing NGC/IC slug, skip.
    if let Some(existing) = row.objname.as_deref().and_then(extract_existing_slug_ref) {
        let hit: Option<i64> = sqlx::query_scalar("select 1::int8 from targets where slug = $1")
            .bind(&existing)
            .fetch_optional(pool)
            .await?;
        if hit.is_some() {
            return Ok(UpsertOutcome::DedupedWithNgc);
        }
    }

    let canonical = row
        .objname
        .clone()
        .unwrap_or_else(|| format!("PGC {}", row.pgc));

    // UPSERT: never overwrite canonical_name on UPDATE; always refresh
    // astro fields. Mirrors the OpenNGC seed contract.
    let inserted: bool = sqlx::query_scalar(
        r#"
        insert into targets (
            slug, canonical_name, kind,
            right_ascension, declination, magnitude_v,
            object_type, major_axis_arcmin, minor_axis_arcmin,
            position_angle_deg, updated_at
        ) values ($1, $2, 'pgc', $3, $4, $5, 'G', $6, $7, $8, now())
        on conflict (slug) do update set
            right_ascension     = excluded.right_ascension,
            declination         = excluded.declination,
            magnitude_v         = excluded.magnitude_v,
            object_type         = excluded.object_type,
            major_axis_arcmin   = excluded.major_axis_arcmin,
            minor_axis_arcmin   = excluded.minor_axis_arcmin,
            position_angle_deg  = excluded.position_angle_deg,
            updated_at          = now()
        returning (xmax = 0)
        "#,
    )
    .bind(&slug)
    .bind(&canonical)
    .bind(row.ra_deg)
    .bind(row.de_deg)
    .bind(row.mag_b)
    .bind(row.major_axis_arcmin)
    .bind(row.minor_axis_arcmin)
    .bind(row.position_angle_deg)
    .fetch_one(pool)
    .await?;

    Ok(if inserted {
        UpsertOutcome::Inserted
    } else {
        UpsertOutcome::Updated
    })
}

#[derive(Default, Debug)]
struct Counts {
    inserted: usize,
    updated: usize,
    deduped_with_ngc: usize,
    skipped_parse: usize,
}

#[derive(Debug)]
enum UpsertOutcome {
    Inserted,
    Updated,
    DedupedWithNgc,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("APP_DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!("DATABASE_URL or APP_DATABASE_URL must be set"))?;
    let pool = sqlx::PgPool::connect(&database_url).await?;

    let csv_path = std::env::var("PGC_DATA_PATH").unwrap_or_else(|_| "data/pgc/pgc.csv".into());

    let file =
        std::fs::File::open(&csv_path).map_err(|e| anyhow::anyhow!("open {}: {}", csv_path, e))?;
    let reader: Box<dyn std::io::Read> = if csv_path.ends_with(".gz") {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    let headers = rdr.headers()?.clone();

    let mut counts = Counts::default();
    for (i, record) in rdr.records().enumerate() {
        let record = record?;
        let row = match parse_csv_row(&record, &headers)? {
            Some(r) => r,
            None => {
                counts.skipped_parse += 1;
                continue;
            }
        };
        match upsert_pgc_row(&pool, &row).await? {
            UpsertOutcome::Inserted => counts.inserted += 1,
            UpsertOutcome::Updated => counts.updated += 1,
            UpsertOutcome::DedupedWithNgc => counts.deduped_with_ngc += 1,
        }
        if (i + 1) % 5000 == 0 {
            tracing::info!(processed = i + 1, ?counts, "seed-pgc progress");
        }
    }

    tracing::info!(
        inserted = counts.inserted,
        updated = counts.updated,
        deduped_with_ngc = counts.deduped_with_ngc,
        skipped_parse = counts.skipped_parse,
        "seed-pgc complete"
    );
    Ok(())
}
