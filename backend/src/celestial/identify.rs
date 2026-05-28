//! Identification entry point. Called from `platesolve::save_result`
//! inside the same transaction so the solve + identification are atomic.
//! See spec §3.

use crate::celestial::confidence::confidence;
use crate::celestial::queries::{arc_distance_deg, cone_search_in_tx};
use crate::error::AppError;
use sqlx::types::BigDecimal;
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

/// `photo_targets.confidence` is `numeric` (BigDecimal). Convert via the
/// string round-trip used elsewhere in the codebase (see equipment::specs).
fn f32_to_decimal(v: f32) -> BigDecimal {
    BigDecimal::from_str(&format!("{v}")).unwrap_or_default()
}

/// Summary returned to the caller (and exposed by the POST recompute
/// handler). `found` is the cone-search result count; `kept` survived
/// the write-time filter; `dropped = found - kept`.
#[derive(Debug, Default, serde::Serialize)]
pub struct IdentifyOutcome {
    pub found: usize,
    pub kept: usize,
    pub dropped: usize,
}

pub async fn identify(
    photo_id: Uuid,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<IdentifyOutcome, AppError> {
    // 1. Read solve telemetry + image size.
    let row = sqlx::query!(
        r#"select ra_deg, dec_deg, platesolve_pixel_scale_arcsec as scale,
                  width, height
             from photos
            where id = $1"#,
        photo_id,
    )
    .fetch_one(&mut **tx)
    .await?;

    // `scale` is `real` in the DB → f32; cast to f64 for the arithmetic
    // below (everything else is f64).
    let (ra, dec, scale, w, h) = match (row.ra_deg, row.dec_deg, row.scale, row.width, row.height) {
        (Some(a), Some(d), Some(s), Some(w), Some(h)) if s > 0.0 && w > 0 && h > 0 => {
            (a, d, f64::from(s), f64::from(w), f64::from(h))
        }
        // Photo lacks the data we need to project a search radius —
        // short-circuit harmlessly rather than fail the surrounding tx.
        _ => return Ok(IdentifyOutcome::default()),
    };

    // 2. Compute search radius from the FOV's half-diagonal.
    let fov_x_deg = w * scale / 3600.0;
    let fov_y_deg = h * scale / 3600.0;
    let half_diag_deg = 0.5 * (fov_x_deg * fov_x_deg + fov_y_deg * fov_y_deg).sqrt();

    // 3. Cone search in the same transaction.
    let candidates = cone_search_in_tx(tx, ra, dec, half_diag_deg).await?;
    let found = candidates.len();

    // 4. Project + filter (write-time F1) + score.
    struct Kept {
        target_id: Uuid,
        confidence: f32,
    }
    let mut kept: Vec<Kept> = Vec::with_capacity(found);
    for c in &candidates {
        let dist = arc_distance_deg(ra, dec, c.right_ascension, c.declination);
        let major_arcmin = f64::from(c.major_axis_arcmin.unwrap_or(0.0));
        let on_screen_px = (major_arcmin * 60.0) / scale;
        // F1: keep named catalog rows unconditionally; drop sub-pixel PGC.
        let keep = on_screen_px >= 0.5
            || matches!(c.kind.as_str(), "messier" | "ngc" | "ic" | "caldwell");
        if !keep {
            continue;
        }
        let conf = confidence(dist, half_diag_deg, on_screen_px, &c.kind, c.magnitude_v);
        kept.push(Kept {
            target_id: c.id,
            confidence: conf,
        });
    }

    let kept_n = kept.len();
    let dropped = found - kept_n;

    // 5. Replace the photo's plate-solve rows (idempotent on re-run).
    sqlx::query!(
        "delete from photo_targets where photo_id = $1 and source = 'plate_solve'",
        photo_id,
    )
    .execute(&mut **tx)
    .await?;

    for k in &kept {
        sqlx::query!(
            r#"insert into photo_targets (photo_id, target_id, source, confidence, is_primary)
                 values ($1, $2, 'plate_solve', $3, false)
                 on conflict (photo_id, target_id) do nothing"#,
            photo_id,
            k.target_id,
            f32_to_decimal(k.confidence),
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(IdentifyOutcome {
        found,
        kept: kept_n,
        dropped,
    })
}
