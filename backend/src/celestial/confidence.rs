//! Pure scoring function — see spec §4.
//!
//! Output is in [0, 1] and combines four sub-scores: how centred the
//! target is in the frame, how big it renders on screen, whether it
//! sits in a "named" catalog (Messier/NGC/IC/Caldwell), and how bright
//! it is. Weights chosen so the centred + sized + named axes carry most
//! of the signal; magnitude is a tiebreaker.

pub fn confidence(
    arc_distance_deg: f64,
    half_diagonal_deg: f64,
    on_screen_size_px: f64,
    kind: &str,
    magnitude_v: Option<f32>,
) -> f32 {
    let center_score = (1.0 - (arc_distance_deg / half_diagonal_deg)).clamp(0.0, 1.0);
    let size_score = (on_screen_size_px / 20.0).clamp(0.0, 1.0);
    let named_bonus: f64 = match kind {
        "messier" | "ngc" | "ic" | "caldwell" => 1.0,
        _ => 0.5,
    };
    let mag_quality: f64 = match magnitude_v {
        None => 0.5,
        Some(m) if m <= 12.0 => 1.0,
        Some(m) => (1.0 - (f64::from(m) - 12.0) / 6.0).max(0.0),
    };
    (0.40 * center_score + 0.30 * size_score + 0.20 * named_bonus + 0.10 * mag_quality) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_centered_named_bright_object_scores_near_one() {
        let c = confidence(0.0, 0.3, 50.0, "messier", Some(6.0));
        assert!(c > 0.9, "expected near 1, got {}", c);
    }

    #[test]
    fn off_center_tiny_unknown_faint_scores_low() {
        let c = confidence(0.29, 0.3, 0.5, "pgc", Some(18.0));
        assert!(c < 0.2, "expected low, got {}", c);
    }

    #[test]
    fn missing_magnitude_treated_as_half() {
        let bright = confidence(0.0, 0.3, 20.0, "ngc", Some(8.0));
        let unknown = confidence(0.0, 0.3, 20.0, "ngc", None);
        assert!(bright > unknown);
        assert!(unknown > 0.7); // still strong; only the mag term degrades.
    }

    #[test]
    fn result_always_in_zero_one_range() {
        for size in [0.0, 0.4, 5.0, 50.0] {
            for dist in [0.0, 0.1, 0.5, 1.0] {
                let c = confidence(dist, 0.5, size, "pgc", Some(15.0));
                assert!((0.0..=1.0).contains(&c), "out of range: {}", c);
            }
        }
    }
}
