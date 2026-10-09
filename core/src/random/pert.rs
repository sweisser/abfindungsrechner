//! PERT distribution: a smooth three-point estimate (min / most likely / max).

use serde::Deserialize;

use super::Rng;

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(try_from = "RawPert")]
pub struct Pert {
    min: f64,
    mode: f64,
    max: f64,
}

/// Unvalidated JSON form.
#[derive(Deserialize)]
struct RawPert {
    min: f64,
    mode: f64,
    max: f64,
}

impl TryFrom<RawPert> for Pert {
    type Error = String;
    fn try_from(r: RawPert) -> Result<Self, String> {
        Pert::new(r.min, r.mode, r.max)
    }
}

impl Pert {
    pub fn new(min: f64, mode: f64, max: f64) -> Result<Self, String> {
        // Written so that NaN fails too.
        if !(min <= mode && mode <= max) {
            return Err(format!("expected min ≤ mode ≤ max, got {min} / {mode} / {max}"));
        }
        Ok(Pert { min, mode, max })
    }

    pub fn mean(&self) -> f64 {
        (self.min + 4.0 * self.mode + self.max) / 6.0
    }

    /// A Beta distribution stretched onto [min, max] with its peak at `mode`.
    pub fn sample(&self, rng: &mut Rng) -> f64 {
        let range = self.max - self.min;
        if range == 0.0 {
            return self.min;
        }
        let a = 1.0 + 4.0 * (self.mode - self.min) / range;
        let b = 1.0 + 4.0 * (self.max - self.mode) / range;
        self.min + range * rng.beta(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(d: Pert) -> Vec<f64> {
        let mut rng = Rng::new(3);
        (0..50_000).map(|_| d.sample(&mut rng)).collect()
    }

    #[test]
    fn rejects_unordered_points() {
        assert!(Pert::new(5.0, 2.0, 10.0).is_err());
        assert!(Pert::new(0.0, 12.0, 10.0).is_err());
        assert!(Pert::new(f64::NAN, 1.0, 2.0).is_err());
    }

    #[test]
    fn mean_weights_the_mode_four_times() {
        assert_eq!(Pert::new(0.0, 3.0, 18.0).unwrap().mean(), 5.0);
    }

    #[test]
    fn samples_stay_in_range_and_match_the_mean() {
        let d = Pert::new(2.0, 4.0, 18.0).unwrap();
        let xs = samples(d);
        assert!(xs.iter().all(|&x| (2.0..=18.0).contains(&x)));
        let m = xs.iter().sum::<f64>() / xs.len() as f64;
        assert!((m - d.mean()).abs() < 0.05, "{m} vs {}", d.mean());
    }

    #[test]
    fn degenerate_range_returns_the_point() {
        let mut rng = Rng::new(1);
        assert_eq!(Pert::new(7.0, 7.0, 7.0).unwrap().sample(&mut rng), 7.0);
    }

    #[test]
    fn deserializes_and_validates_json() {
        let ok: Pert = serde_json::from_str(r#"{"min":1,"mode":2,"max":3}"#).unwrap();
        assert_eq!(ok, Pert::new(1.0, 2.0, 3.0).unwrap());
        assert!(serde_json::from_str::<Pert>(r#"{"min":3,"mode":2,"max":1}"#).is_err());
    }
}
