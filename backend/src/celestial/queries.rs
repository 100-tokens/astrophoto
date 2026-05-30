//! Cone-search SQL helpers + spherical distance.
//!
//! Two-step query: a bounding box on `(declination, right_ascension)`
//! pulled by the `targets_radec_idx` partial index, then an exact
//! haversine-distance filter in Rust to drop the corners.
//! Handles the RA 0/360 wrap by emitting two `between` clauses when the
//! window crosses zero.

use crate::error::AppError;
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow)]
pub struct CandidateRow {
    pub id: uuid::Uuid,
    pub slug: String,
    pub canonical_name: String,
    pub kind: String,
    pub object_type: Option<String>,
    pub magnitude_v: Option<f32>,
    pub right_ascension: f64,
    pub declination: f64,
    pub major_axis_arcmin: Option<f32>,
    pub minor_axis_arcmin: Option<f32>,
    pub position_angle_deg: Option<f32>,
}

/// Great-circle distance between two RA/Dec points, in degrees.
/// Uses the standard spherical-law-of-cosines formulation, clamped to
/// guard against floating-point overshoots that would make `acos` return NaN.
pub fn arc_distance_deg(a1_deg: f64, d1_deg: f64, a2_deg: f64, d2_deg: f64) -> f64 {
    let (a1, d1) = (a1_deg.to_radians(), d1_deg.to_radians());
    let (a2, d2) = (a2_deg.to_radians(), d2_deg.to_radians());
    let cos_c = d1.sin() * d2.sin() + d1.cos() * d2.cos() * (a1 - a2).cos();
    cos_c.clamp(-1.0, 1.0).acos().to_degrees()
}

const CONE_SEARCH_SQL_SINGLE: &str = r#"
    select id, slug, canonical_name, kind, object_type, magnitude_v,
           right_ascension, declination,
           major_axis_arcmin, minor_axis_arcmin, position_angle_deg
      from targets
     where right_ascension is not null and declination is not null
       and declination between $1 and $2
       and right_ascension between $3 and $4
"#;

const CONE_SEARCH_SQL_WRAP: &str = r#"
    select id, slug, canonical_name, kind, object_type, magnitude_v,
           right_ascension, declination,
           major_axis_arcmin, minor_axis_arcmin, position_angle_deg
      from targets
     where right_ascension is not null and declination is not null
       and declination between $1 and $2
       and (right_ascension between $3 and $4
         or right_ascension between $5 and $6)
"#;

/// Compute the (dec_min, dec_max, ra-window) bounding box for a search
/// of radius `radius_deg` around `(ra_deg, dec_deg)`. Returns either
/// `Single(ra_min, ra_max)` or `Wrap(lo1, hi1, lo2, hi2)` when the
/// window crosses RA 0/360.
enum RaWindow {
    Single(f64, f64),
    Wrap(f64, f64, f64, f64),
}

fn ra_window(ra_deg: f64, dec_deg: f64, radius_deg: f64) -> RaWindow {
    let cos_dec = (dec_deg.to_radians().cos()).abs().max(1e-6);
    let ra_half = (radius_deg / cos_dec).min(180.0);
    let ra_min = ra_deg - ra_half;
    let ra_max = ra_deg + ra_half;
    if ra_min < 0.0 {
        RaWindow::Wrap(0.0, ra_max, ra_min + 360.0, 360.0)
    } else if ra_max > 360.0 {
        RaWindow::Wrap(ra_min, 360.0, 0.0, ra_max - 360.0)
    } else {
        RaWindow::Single(ra_min, ra_max)
    }
}

pub async fn cone_search(
    pool: &PgPool,
    ra_deg: f64,
    dec_deg: f64,
    radius_deg: f64,
) -> Result<Vec<CandidateRow>, AppError> {
    let dec_min = dec_deg - radius_deg;
    let dec_max = dec_deg + radius_deg;
    let rows: Vec<CandidateRow> = match ra_window(ra_deg, dec_deg, radius_deg) {
        RaWindow::Single(lo, hi) => {
            sqlx::query_as::<_, CandidateRow>(CONE_SEARCH_SQL_SINGLE)
                .bind(dec_min)
                .bind(dec_max)
                .bind(lo)
                .bind(hi)
                .fetch_all(pool)
                .await?
        }
        RaWindow::Wrap(lo1, hi1, lo2, hi2) => {
            sqlx::query_as::<_, CandidateRow>(CONE_SEARCH_SQL_WRAP)
                .bind(dec_min)
                .bind(dec_max)
                .bind(lo1)
                .bind(hi1)
                .bind(lo2)
                .bind(hi2)
                .fetch_all(pool)
                .await?
        }
    };

    // Exact filter — drop the corners introduced by the bounding box.
    Ok(rows
        .into_iter()
        .filter(|r| {
            arc_distance_deg(ra_deg, dec_deg, r.right_ascension, r.declination) <= radius_deg
        })
        .collect())
}

/// Same as `cone_search`, but executes against a transaction so the read
/// joins the same atomic unit as the subsequent `photo_targets` write in
/// `identify::identify`.
pub async fn cone_search_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ra_deg: f64,
    dec_deg: f64,
    radius_deg: f64,
) -> Result<Vec<CandidateRow>, AppError> {
    let dec_min = dec_deg - radius_deg;
    let dec_max = dec_deg + radius_deg;
    let rows: Vec<CandidateRow> = match ra_window(ra_deg, dec_deg, radius_deg) {
        RaWindow::Single(lo, hi) => {
            sqlx::query_as::<_, CandidateRow>(CONE_SEARCH_SQL_SINGLE)
                .bind(dec_min)
                .bind(dec_max)
                .bind(lo)
                .bind(hi)
                .fetch_all(&mut **tx)
                .await?
        }
        RaWindow::Wrap(lo1, hi1, lo2, hi2) => {
            sqlx::query_as::<_, CandidateRow>(CONE_SEARCH_SQL_WRAP)
                .bind(dec_min)
                .bind(dec_max)
                .bind(lo1)
                .bind(hi1)
                .bind(lo2)
                .bind(hi2)
                .fetch_all(&mut **tx)
                .await?
        }
    };

    Ok(rows
        .into_iter()
        .filter(|r| {
            arc_distance_deg(ra_deg, dec_deg, r.right_ascension, r.declination) <= radius_deg
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_distance_known_pairs() {
        // M31 (10.685, 41.269) ↔ M33 (23.462, 30.660) ≈ 14.7°
        let d = arc_distance_deg(10.685, 41.269, 23.462, 30.660);
        assert!((d - 14.7).abs() < 0.2, "got {}", d);
    }

    #[test]
    fn arc_distance_same_point_is_zero() {
        let d = arc_distance_deg(180.0, 0.0, 180.0, 0.0);
        assert!(d < 1e-9);
    }

    #[test]
    fn arc_distance_antipode_is_180() {
        let d = arc_distance_deg(0.0, 0.0, 180.0, 0.0);
        assert!((d - 180.0).abs() < 1e-6);
    }
}
