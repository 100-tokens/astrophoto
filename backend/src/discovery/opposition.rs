//! Opposition / midnight-culmination date for fixed deep-sky objects.
//!
//! For an object at a fixed right ascension, "opposition" — sitting opposite
//! the Sun as seen from Earth — coincides with its meridian transit (upper
//! culmination) happening at local solar midnight: the best night of the year
//! to observe it. For a fixed RA the date depends only on the Sun's position
//! and is stable to ~±1 day year-to-year (SEDS publishes a single per-object
//! "Midnight Culmination Date": M31 = Oct 1, M13 = Jun 1, M1 = Dec 14). We
//! therefore cache it per target as a day-of-year in `targets.opposition_doy`
//! and present it as an *approximate* date.
//!
//! Math: the closed-form low-precision Sun position from the USNO /
//! Astronomical Almanac (equivalent to Meeus ch.25's equation-of-center) is
//! accurate to ~36″ over 1950–2050 — roughly 100× finer than the one-day
//! resolution this feature resolves, so no ephemeris dependency is needed.
//! Opposition is the instant the Sun's apparent RA equals (object RA − 180°);
//! we root-find that instant within a reference non-leap year and return its
//! day-of-year (1..=365).
//!
//! `opposition_doy` is a denormalised cache of `f(right_ascension)`. Every
//! writer of `targets.right_ascension` (the `seed-targets` and `seed-pgc`
//! binaries) must recompute it in the same statement, and [`backfill_missing`]
//! fills it for rows written before this column existed. Mirrors the
//! `photos.filters` / `photo_filters` cache contract in CLAUDE.md.

use sqlx::PgPool;
use uuid::Uuid;

/// Reference (non-leap) year for the day-of-year mapping. The opposition date
/// drifts well under a day across nearby years, so pinning one keeps doy↔date
/// stable and consistent with the month/day the frontend renders. 2025 is the
/// nearest non-leap year, which keeps day-of-year ↔ calendar 1:1 (no Feb 29).
const REF_YEAR: i32 = 2025;

/// Julian Day for a Gregorian calendar date (Meeus, *Astronomical Algorithms*
/// ch. 7). `day` may carry a fractional part (e.g. 1.5 = noon on the 1st).
fn julian_day(year: i32, month: i32, day: f64) -> f64 {
    let (y, m) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y as f64 + 4716.0)).floor() + (30.6001 * (m as f64 + 1.0)).floor() + day + b - 1524.5
}

/// The Sun's apparent right ascension in degrees `[0, 360)` for a given Julian
/// Day, via the USNO low-precision formula
/// (<https://aa.usno.navy.mil/faq/sun_approx>).
fn sun_apparent_ra_deg(jd: f64) -> f64 {
    let d = jd - 2451545.0;
    // Mean longitude (deg) and mean anomaly (rad) of the Sun.
    let q = 280.459 + 0.98564736 * d;
    let g = (357.529 + 0.98560028 * d).to_radians();
    // Apparent ecliptic longitude (deg → rad) and obliquity (deg → rad).
    let lambda = (q + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
    let eps = (23.439 - 0.00000036 * d).to_radians();
    // RA in the same quadrant as λ via atan2; normalise to [0, 360).
    (eps.cos() * lambda.sin())
        .atan2(lambda.cos())
        .to_degrees()
        .rem_euclid(360.0)
}

/// Smallest signed difference `a − b` of two angles, in `(-180, 180]` degrees.
fn signed_delta_deg(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0)) - 180.0
}

/// Day-of-year (1..=365, in the [`REF_YEAR`] non-leap calendar) on which a
/// fixed object at `ra_deg` (J2000 right ascension, degrees) reaches opposition
/// / midnight culmination — its best-observation date.
///
/// Finds the instant the Sun's apparent RA equals `ra_deg − 180°` by scanning
/// a full year for the single upward zero-crossing of the angular difference,
/// then linearly interpolating to sub-day precision. `ra_deg` outside the
/// canonical `[0, 360)` range is wrapped.
pub fn midnight_culmination_doy(ra_deg: f64) -> i16 {
    let target = (ra_deg - 180.0).rem_euclid(360.0);
    let base = julian_day(REF_YEAR, 1, 1.0); // REF_YEAR-01-01 00:00 UT == doy 1.

    // f(n) = signed(Sun RA on day n − target). Over one year the Sun's RA
    // sweeps 360° once, so f has exactly one *upward* crossing (the real
    // opposition); the ±180° wrap of `signed_delta_deg` shows up as a large
    // downward jump, which the `< 180.0` step guard rejects.
    let f = |day_index: f64| signed_delta_deg(sun_apparent_ra_deg(base + day_index), target);

    for i in 0..366 {
        let f0 = f(i as f64);
        let f1 = f(i as f64 + 1.0);
        if f0 <= 0.0 && f1 > 0.0 && (f1 - f0) < 180.0 {
            let frac = -f0 / (f1 - f0); // 0..1 within the day
            // `base` is day-of-year 1, so the calendar date containing the
            // instant is floor(days since base) + 1.
            let doy = (i as f64 + frac).floor() as i32 + 1;
            // Wrap a crossing that lands on the Dec 31 → Jan 1 seam back into 1..=365.
            return ((doy - 1).rem_euclid(365) + 1) as i16;
        }
    }

    // Unreachable: every target value has one upward crossing per year. Fall
    // back to a coarse nearest-day scan rather than panicking.
    let mut best_day = 0i32;
    let mut best_abs = f64::INFINITY;
    for i in 0..365 {
        let d = f(i as f64).abs();
        if d < best_abs {
            best_abs = d;
            best_day = i;
        }
    }
    (best_day + 1) as i16
}

/// Circular day-of-year distance (`0..=182`) between two days in the non-leap
/// reference calendar — how far apart they are going around the year the short
/// way (e.g. doy 5 vs doy 360 → 10, not 355). Used to rank targets by how near
/// their opposition is to a given date for the "optimal now" sort; the SQL
/// `ORDER BY` in `target_index` mirrors this exact expression, and this helper
/// computes the matching keyset cursor value so the two never drift.
pub fn circular_doy_distance(a: i32, b: i32) -> i32 {
    let d = (a - b).abs();
    d.min(365 - d)
}

/// Populates `targets.opposition_doy` for every row that has a right ascension
/// but no cached opposition date yet. Idempotent: after the first run no NULL
/// rows remain, so subsequent boots fetch nothing and update nothing. This is
/// the one-time "rebuild for existing rows" — the seeds keep new rows fresh.
/// Returns the number of rows filled.
pub async fn backfill_missing(pool: &PgPool) -> sqlx::Result<u64> {
    let rows = sqlx::query!(
        r#"
        select id as "id!", right_ascension as "ra!"
          from targets
         where opposition_doy is null and right_ascension is not null
        "#
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let doys: Vec<i16> = rows
        .iter()
        .map(|r| midnight_culmination_doy(r.ra))
        .collect();

    let res = sqlx::query!(
        r#"
        update targets t
           set opposition_doy = d.doy
          from unnest($1::uuid[], $2::int2[]) as d(id, doy)
         where t.id = d.id
        "#,
        &ids,
        &doys
    )
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Converts an `Hh Mm` right ascension to degrees (RA hours × 15).
    fn ra_hm(h: f64, m: f64) -> f64 {
        (h + m / 60.0) * 15.0
    }

    // Reference anchors from SEDS' published "Midnight Culmination Date"
    // (http://www.messier.seds.org/xtra/supp/midnite.html), epoch 2000.0.
    // doy in a non-leap year: Oct 1 = 274, Jun 1 = 152, Dec 14 = 348.

    #[test]
    fn m31_opposition_is_early_october() {
        // M31 RA 00h42.7m. SEDS lists Oct 1 (doy 274), but SEDS keys to the
        // *mean* sun (clock-midnight culmination); we compute *true* opposition
        // (apparent Sun RA = object RA − 180°). Near the autumnal equinox the
        // equation of time is ~+12 min ≈ 3°, so apparent opposition lands ~Oct 4
        // (doy 277). Both round to "early October", which is all the UI shows.
        let doy = midnight_culmination_doy(ra_hm(0.0, 42.7));
        assert!(
            (doy - 274).abs() <= 4,
            "M31 doy {doy}, want early Oct (~274)"
        );
    }

    #[test]
    fn m13_culminates_at_midnight_around_jun_1() {
        // M13 RA 16h41.7m. SEDS: Jun 1 (doy 152).
        let doy = midnight_culmination_doy(ra_hm(16.0, 41.7));
        assert!((doy - 152).abs() <= 2, "M13 doy {doy}, want ~152 (Jun 1)");
    }

    #[test]
    fn m1_culminates_at_midnight_around_dec_14() {
        // M1 RA 05h34.5m. SEDS: Dec 14 (doy 348).
        let doy = midnight_culmination_doy(ra_hm(5.0, 34.5));
        assert!((doy - 348).abs() <= 2, "M1 doy {doy}, want ~348 (Dec 14)");
    }

    #[test]
    fn opposition_is_about_six_months_from_the_suns_ra() {
        // An object whose RA equals the Sun's RA on a date is at *conjunction*
        // then; opposition is ~half a year away. RA 0h (vernal-equinox point):
        // Sun shares that RA ~Mar 20, so opposition is ~Sep 22-23 (doy ~265).
        let doy = midnight_culmination_doy(0.0);
        assert!((doy - 266).abs() <= 3, "RA 0h doy {doy}, want ~Sep 22");
    }

    #[test]
    fn result_is_always_a_valid_day_of_year() {
        let mut deg = 0.0;
        while deg < 360.0 {
            let doy = midnight_culmination_doy(deg);
            assert!(
                (1..=365).contains(&doy),
                "deg {deg} → doy {doy} out of range"
            );
            deg += 0.5;
        }
    }

    #[test]
    fn doy_increases_monotonically_with_ra_modulo_wrap() {
        // As RA increases, opposition date advances through the calendar (with
        // exactly one wrap around the year). Check that consecutive samples
        // step forward by a small, roughly-constant amount apart from one seam.
        let mut wraps = 0;
        let mut prev = midnight_culmination_doy(0.0);
        let mut deg = 1.0;
        while deg < 360.0 {
            let cur = midnight_culmination_doy(deg);
            if cur < prev {
                wraps += 1;
            }
            prev = cur;
            deg += 1.0;
        }
        assert_eq!(
            wraps, 1,
            "expected exactly one year-boundary wrap, got {wraps}"
        );
    }

    #[test]
    fn julian_day_matches_known_epoch() {
        // 2025-01-01 00:00 UT is JD 2460676.5; J2000.0 is JD 2451545.0 (noon).
        assert!((julian_day(2025, 1, 1.0) - 2460676.5).abs() < 1e-6);
        assert!((julian_day(2000, 1, 1.5) - 2451545.0).abs() < 1e-6);
    }

    #[test]
    fn circular_distance_is_zero_at_the_same_day() {
        assert_eq!(circular_doy_distance(200, 200), 0);
    }

    #[test]
    fn circular_distance_wraps_the_year_boundary() {
        // doy 5 (early Jan) and doy 360 (late Dec) are 10 days apart, not 355.
        assert_eq!(circular_doy_distance(5, 360), 10);
        assert_eq!(circular_doy_distance(360, 5), 10); // symmetric
    }

    #[test]
    fn circular_distance_is_symmetric_and_capped_at_half_year() {
        assert_eq!(circular_doy_distance(1, 100), circular_doy_distance(100, 1));
        // Opposite sides of the year are at most ~half a year apart.
        assert_eq!(circular_doy_distance(1, 183), 182);
        let mut max = 0;
        for d in 1..=365 {
            max = max.max(circular_doy_distance(1, d));
        }
        assert_eq!(max, 182);
    }
}
