//! Parses pinned OpenNGC CSVs and UPSERTs into `targets`. Idempotent.
//! See docs/superpowers/specs/2026-05-06-celestial-objects-design.md

use anyhow::Result;

#[derive(Debug, PartialEq)]
pub struct OpenNgcRow {
    pub name: String, // e.g. "NGC0224"
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

pub fn parse_csv_row(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
) -> Result<OpenNgcRow> {
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
        .map(|s| {
            s.split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(OpenNgcRow {
        name,
        messier_num,
        ra_deg,
        dec_deg,
        object_type,
        constellation,
        magnitude_v,
        major_axis_arcmin,
        minor_axis_arcmin,
        common_names,
    })
}

/// Parse "00:42:44.330" → degrees in [0, 360).
fn parse_ra_sexagesimal(s: &str) -> Result<f64> {
    let mut parts = s.split(':');
    let h: f64 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("RA empty"))?
        .parse()?;
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
    let d: f64 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Dec empty"))?
        .parse()?;
    let m: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
    let sec: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
    Ok(sign * (d + m / 60.0 + sec / 3600.0))
}

const KEEP_MANUAL_META: &[&str] = &["ic-434"]; // verified by Task 1 against OpenNGC

#[tokio::main]
async fn main() -> Result<()> {
    use std::path::PathBuf;
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("APP_DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!("DATABASE_URL or APP_DATABASE_URL must be set"))?;

    let pool = sqlx::PgPool::connect(&database_url).await?;

    let data_dir = std::env::var("OPENNGC_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/openngc"));

    let mut counts = Counts::default();

    process_csv(&pool, &data_dir.join("NGC.csv"), &mut counts).await?;
    process_csv(&pool, &data_dir.join("addendum.csv"), &mut counts).await?;

    tracing::info!(
        upserts = counts.upserts,
        skipped_subcomponent = counts.skipped_subcomponent,
        skipped_unknown_prefix = counts.skipped_unknown_prefix,
        skipped_duplicate = counts.skipped_duplicate,
        "seed-targets complete"
    );
    Ok(())
}

#[derive(Default, Debug)]
struct Counts {
    upserts: usize,
    skipped_subcomponent: usize,
    skipped_unknown_prefix: usize,
    skipped_duplicate: usize,
}

async fn process_csv(
    pool: &sqlx::PgPool,
    path: &std::path::Path,
    counts: &mut Counts,
) -> Result<()> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_path(path)?;
    let headers = rdr.headers()?.clone();

    for record in rdr.records() {
        let record = record?;
        let row = match parse_csv_row(&record, &headers) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("parse error on row: {e}");
                continue;
            }
        };
        match compute_slug(&row) {
            SlugDecision::Slug(slug) => {
                upsert_target(pool, &slug, &row, KEEP_MANUAL_META).await?;
                counts.upserts += 1;
            }
            SlugDecision::Skip(SkipReason::SubcomponentSuffix) => counts.skipped_subcomponent += 1,
            SlugDecision::Skip(SkipReason::UnknownPrefix) => counts.skipped_unknown_prefix += 1,
            SlugDecision::Skip(SkipReason::Duplicate) => counts.skipped_duplicate += 1,
        }
    }
    Ok(())
}

pub async fn upsert_target(
    pool: &sqlx::PgPool,
    slug: &str,
    row: &OpenNgcRow,
    keep_manual_meta: &[&str],
) -> anyhow::Result<()> {
    let in_skip_list = keep_manual_meta.contains(&slug);

    // Aliases that should always be appended (idempotent dedup happens in SQL).
    let mut alias_additions: Vec<String> = Vec::new();
    if let Some(m) = row.messier_num {
        alias_additions.push(format!("M {}", m));
    }
    if let Some(rest) = row.name.strip_prefix("NGC")
        && let Ok(n) = rest.parse::<u32>()
    {
        alias_additions.push(format!("NGC {}", n));
    }
    if let Some(rest) = row.name.strip_prefix("IC")
        && let Ok(n) = rest.parse::<u32>()
    {
        alias_additions.push(format!("IC {}", n));
    }

    let kind = if slug.starts_with('m') && !slug.starts_with("messier") {
        "messier"
    } else if slug.starts_with("ngc-") {
        "ngc"
    } else if slug.starts_with("ic-") {
        "ic"
    } else {
        "other"
    };

    // Canonical name on insert: prefer the first common name when OpenNGC has one.
    // Otherwise format the raw `Name` field — strip the zero-padding so "NGC0224"
    // becomes "NGC 224" rather than displayed verbatim. The slug ("ngc-224") is
    // already de-padded; the canonical needs the same treatment so cards look like
    // "NGC-224 / NGC 224 / Galaxy" instead of "NGC-224 / NGC0224 / Galaxy".
    let canonical_on_insert = row
        .common_names
        .first()
        .cloned()
        .unwrap_or_else(|| pretty_name_fallback(&row.name));

    let constellation_3 = row
        .constellation
        .as_deref()
        .map(|c| c.chars().take(3).collect::<String>());

    if in_skip_list {
        // INSERT-only path: never touches astro fields on existing row; aliases still extended.
        sqlx::query!(
            r#"
            insert into targets (slug, canonical_name, aliases, kind)
            values ($1, $2, $3, $4)
            on conflict (slug) do update set
              aliases = (
                select array(select distinct unnest(targets.aliases || $3))
              )
            "#,
            slug,
            canonical_on_insert,
            &alias_additions,
            kind,
        )
        .execute(pool)
        .await?;
        return Ok(());
    }

    // canonical_name policy on UPSERT: keep user-curated names (e.g. "Andromeda
    // Galaxy" set by migration 0010) and avoid downgrading to a bare catalog id.
    // The exception is the auto-generated "Messier N" placeholder also from 0010
    // — those should be promoted to OpenNGC's common name when one exists, so
    // M1 displays "Crab Nebula", M81 "Bode's Galaxy", etc. The regex tests
    // isolate exactly that placeholder; manual overrides won't match.
    sqlx::query!(
        r#"
        insert into targets (
            slug, canonical_name, aliases, kind,
            right_ascension, declination, magnitude_v, object_type,
            constellation, major_axis_arcmin, minor_axis_arcmin, updated_at
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now())
        on conflict (slug) do update set
            canonical_name = case
                when targets.canonical_name ~ '^Messier [0-9]+$'
                     and excluded.canonical_name !~ '^(NGC|IC) [0-9]+$'
                then excluded.canonical_name
                else targets.canonical_name
            end,
            right_ascension   = excluded.right_ascension,
            declination       = excluded.declination,
            magnitude_v       = excluded.magnitude_v,
            object_type       = excluded.object_type,
            constellation     = excluded.constellation,
            major_axis_arcmin = excluded.major_axis_arcmin,
            minor_axis_arcmin = excluded.minor_axis_arcmin,
            aliases = (
                select array(select distinct unnest(targets.aliases || $3))
            ),
            updated_at = now()
        "#,
        slug,
        canonical_on_insert,
        &alias_additions,
        kind,
        row.ra_deg,
        row.dec_deg,
        row.magnitude_v,
        row.object_type,
        constellation_3,
        row.major_axis_arcmin,
        row.minor_axis_arcmin,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod upsert_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use sqlx::PgPool;
    use testcontainers::ImageExt;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres as PgImage;

    async fn fresh_pool() -> (PgPool, testcontainers::ContainerAsync<PgImage>) {
        let pg = PgImage::default().with_tag("16-alpine").start().await.unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, pg)
    }

    fn fixture(name: &str, m: Option<u32>, ra: f64, dec: f64) -> OpenNgcRow {
        OpenNgcRow {
            name: name.into(),
            messier_num: m,
            ra_deg: Some(ra),
            dec_deg: Some(dec),
            object_type: Some("G".into()),
            constellation: Some("And".into()),
            magnitude_v: Some(3.4),
            major_axis_arcmin: Some(190.0),
            minor_axis_arcmin: Some(60.0),
            common_names: vec!["Andromeda Galaxy".into()],
        }
    }

    #[tokio::test]
    async fn updates_existing_messier_keeps_canonical_name() {
        let (pool, _pg) = fresh_pool().await;
        // Migration 0010 already seeded m31 with canonical_name="Andromeda Galaxy".
        let row = fixture("NGC0224", Some(31), 10.68, 41.27);
        upsert_target(&pool, "m31", &row, &[]).await.unwrap();

        let r = sqlx::query!(
            "select canonical_name, right_ascension, object_type, aliases from targets where slug='m31'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(r.canonical_name, "Andromeda Galaxy");
        assert!((r.right_ascension.unwrap() - 10.68).abs() < 0.01);
        assert_eq!(r.object_type.as_deref(), Some("G"));
        assert!(r.aliases.iter().any(|a| a == "NGC 224"));
    }

    #[tokio::test]
    async fn skip_list_blocks_meta_update() {
        let (pool, _pg) = fresh_pool().await;
        // Migration 0010 seeded m45 with canonical_name='Pleiades' and no astro fields.
        let row = OpenNgcRow {
            name: "M045-fake".into(),
            messier_num: Some(45),
            ra_deg: Some(56.6),
            dec_deg: Some(24.1),
            object_type: Some("HII".into()),
            constellation: Some("Tau".into()),
            magnitude_v: Some(1.6),
            major_axis_arcmin: Some(110.0),
            minor_axis_arcmin: Some(110.0),
            common_names: vec!["Maia Nebula bogus".into()],
        };
        upsert_target(&pool, "m45", &row, &["m45"]).await.unwrap();

        let r = sqlx::query!(
            "select canonical_name, right_ascension, object_type from targets where slug='m45'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(r.canonical_name, "Pleiades");
        assert!(
            r.right_ascension.is_none(),
            "skip-list must block astro update"
        );
        assert!(r.object_type.is_none(), "skip-list must block astro update");
    }

    #[tokio::test]
    async fn idempotent_double_run() {
        let (pool, _pg) = fresh_pool().await;
        let row = fixture("NGC0224", Some(31), 10.68, 41.27);
        upsert_target(&pool, "m31", &row, &[]).await.unwrap();
        upsert_target(&pool, "m31", &row, &[]).await.unwrap();

        let count: i64 =
            sqlx::query_scalar!("select count(*) as \"c!\" from targets where slug='m31'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);

        let r = sqlx::query!("select aliases from targets where slug='m31'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let ngc_count = r.aliases.iter().filter(|a| *a == "NGC 224").count();
        assert_eq!(ngc_count, 1, "alias must be deduped on idempotent re-run");
    }
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

/// Format a raw OpenNGC `Name` ("NGC0224", "IC0434") into a display-friendly form
/// ("NGC 224", "IC 434"). Used as canonical_name fallback when the row has no
/// CommonNames. Falls through unchanged if the name doesn't match NGC/IC/M.
fn pretty_name_fallback(name: &str) -> String {
    for prefix in ["NGC", "IC", "M"] {
        if let Some(rest) = name.strip_prefix(prefix)
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
            && let Ok(n) = rest.parse::<u32>()
        {
            return format!("{prefix} {n}");
        }
    }
    name.to_string()
}

#[cfg(test)]
mod pretty_name_tests {
    use super::pretty_name_fallback;

    #[test]
    fn strips_zero_padding_ngc() {
        assert_eq!(pretty_name_fallback("NGC0224"), "NGC 224");
        assert_eq!(pretty_name_fallback("NGC7000"), "NGC 7000");
    }
    #[test]
    fn strips_zero_padding_ic() {
        assert_eq!(pretty_name_fallback("IC0434"), "IC 434");
    }
    #[test]
    fn passes_through_non_numeric() {
        assert_eq!(pretty_name_fallback("Mel022"), "Mel022");
        assert_eq!(pretty_name_fallback("Cr399"), "Cr399");
    }
}

#[cfg(test)]
mod slug_tests {
    use super::*;

    fn row(name: &str, m: Option<u32>, object_type: Option<&str>) -> OpenNgcRow {
        OpenNgcRow {
            name: name.to_string(),
            messier_num: m,
            ra_deg: None,
            dec_deg: None,
            object_type: object_type.map(String::from),
            constellation: None,
            magnitude_v: None,
            major_axis_arcmin: None,
            minor_axis_arcmin: None,
            common_names: vec![],
        }
    }

    #[test]
    fn messier_slug() {
        assert_eq!(
            compute_slug(&row("NGC0224", Some(31), Some("G"))),
            SlugDecision::Slug("m31".into())
        );
    }
    #[test]
    fn ngc_slug_strips_zeros() {
        assert_eq!(
            compute_slug(&row("NGC0224", None, Some("G"))),
            SlugDecision::Slug("ngc-224".into())
        );
        assert_eq!(
            compute_slug(&row("NGC7000", None, Some("HII"))),
            SlugDecision::Slug("ngc-7000".into())
        );
    }
    #[test]
    fn ic_slug() {
        assert_eq!(
            compute_slug(&row("IC0434", None, Some("HII"))),
            SlugDecision::Slug("ic-434".into())
        );
    }
    #[test]
    fn skips_subcomponent() {
        assert_eq!(
            compute_slug(&row("NGC5128A", None, Some("G"))),
            SlugDecision::Skip(SkipReason::SubcomponentSuffix)
        );
        assert_eq!(
            compute_slug(&row("NGC0292A", None, Some("G"))),
            SlugDecision::Skip(SkipReason::SubcomponentSuffix)
        );
    }
    #[test]
    fn skips_unknown_prefix() {
        assert_eq!(
            compute_slug(&row("PGC1234", None, Some("G"))),
            SlugDecision::Skip(SkipReason::UnknownPrefix)
        );
        assert_eq!(
            compute_slug(&row("B033", None, Some("DrkN"))),
            SlugDecision::Skip(SkipReason::UnknownPrefix)
        );
    }
    #[test]
    fn skips_dup_type() {
        // M102 in addendum: M=101, Type=Dup. Must skip BEFORE attempting slug from M=101,
        // otherwise we'd overwrite NGC5457 (M=101, Type=G, Pinwheel Galaxy).
        assert_eq!(
            compute_slug(&row("M102", Some(101), Some("Dup"))),
            SlugDecision::Skip(SkipReason::Duplicate)
        );
        // Sanity check: a Dup row without an M number still skips, regardless of name prefix.
        assert_eq!(
            compute_slug(&row("NGC1234", None, Some("Dup"))),
            SlugDecision::Skip(SkipReason::Duplicate)
        );
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
        let headers = rec(&[
            "Name",
            "Type",
            "RA",
            "Dec",
            "Const",
            "MajAx",
            "MinAx",
            "V-Mag",
            "M",
            "Common names",
        ]);
        let row = rec(&[
            "NGC0224",
            "G",
            "00:42:44.330",
            "+41:16:09.40",
            "And",
            "190.0",
            "60.0",
            "3.44",
            "31",
            "Andromeda Galaxy,M 31",
        ]);
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
        let headers = rec(&[
            "Name",
            "Type",
            "RA",
            "Dec",
            "Const",
            "MajAx",
            "MinAx",
            "V-Mag",
            "M",
            "Common names",
        ]);
        let row = rec(&[
            "NGC1234",
            "G",
            "02:00:00",
            "+10:00:00",
            "Tau",
            "",
            "",
            "",
            "",
            "",
        ]);
        let parsed = parse_csv_row(&row, &headers).unwrap();
        assert_eq!(parsed.magnitude_v, None);
        assert_eq!(parsed.major_axis_arcmin, None);
        assert_eq!(parsed.messier_num, None);
        assert!(parsed.common_names.is_empty());
    }
}
