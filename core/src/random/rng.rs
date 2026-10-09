//! Small seedable random number generator (SplitMix64) with the samplers the model needs.

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        // The top 53 bits fill the mantissa exactly.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal N(0, 1) (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u = 1.0 - self.uniform(); // in (0, 1], so ln(u) is finite
        let v = self.uniform();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }

    /// Gamma(shape, 1) (Marsaglia-Tsang).
    pub fn gamma(&mut self, shape: f64) -> f64 {
        if shape < 1.0 {
            // Boost: Gamma(a) = Gamma(a + 1) * U^(1/a)
            let u = 1.0 - self.uniform();
            return self.gamma(shape + 1.0) * u.powf(1.0 / shape);
        }
        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();
        loop {
            let x = self.normal();
            let v = (1.0 + c * x).powi(3);
            if v <= 0.0 {
                continue;
            }
            let u = 1.0 - self.uniform();
            if u.ln() < 0.5 * x * x + d - d * v + d * v.ln() {
                return d * v;
            }
        }
    }

    /// Beta(a, b) in [0, 1].
    pub fn beta(&mut self, a: f64, b: f64) -> f64 {
        let x = self.gamma(a);
        let y = self.gamma(b);
        x / (x + y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 100_000;

    fn mean_var(xs: &[f64]) -> (f64, f64) {
        let m = xs.iter().sum::<f64>() / xs.len() as f64;
        let v = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64;
        (m, v)
    }

    fn draws(seed: u64, f: impl Fn(&mut Rng) -> f64) -> Vec<f64> {
        let mut r = Rng::new(seed);
        (0..N).map(|_| f(&mut r)).collect()
    }

    #[test]
    fn same_seed_gives_same_sequence() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        for _ in 0..100 {
            assert_eq!(a.uniform(), b.uniform());
        }
    }

    #[test]
    fn different_seeds_give_different_sequences() {
        let (mut a, mut b) = (Rng::new(1), Rng::new(2));
        assert_ne!(a.uniform(), b.uniform());
    }

    #[test]
    fn uniform_is_in_unit_interval_with_mean_one_half() {
        let xs = draws(7, Rng::uniform);
        assert!(xs.iter().all(|&x| (0.0..1.0).contains(&x)));
        let (m, v) = mean_var(&xs);
        assert!((m - 0.5).abs() < 0.01, "{m}");
        assert!((v - 1.0 / 12.0).abs() < 0.002, "{v}");
    }

    #[test]
    fn normal_has_mean_zero_and_unit_variance() {
        let (m, v) = mean_var(&draws(8, Rng::normal));
        assert!(m.abs() < 0.02, "{m}");
        assert!((v - 1.0).abs() < 0.02, "{v}");
    }

    #[test]
    fn gamma_has_mean_and_variance_equal_to_shape() {
        for shape in [0.5, 1.0, 3.0, 12.0] {
            let xs = draws(9, |r| r.gamma(shape));
            assert!(xs.iter().all(|&x| x >= 0.0));
            let (m, v) = mean_var(&xs);
            assert!((m / shape - 1.0).abs() < 0.03, "shape {shape}: mean {m}");
            assert!((v / shape - 1.0).abs() < 0.05, "shape {shape}: var {v}");
        }
    }

    #[test]
    fn beta_is_in_unit_interval_with_expected_mean() {
        for (a, b) in [(2.0, 5.0), (4.0, 4.0), (1.5, 0.8)] {
            let xs = draws(10, |r| r.beta(a, b));
            assert!(xs.iter().all(|&x| (0.0..=1.0).contains(&x)));
            let (m, _) = mean_var(&xs);
            assert!((m - a / (a + b)).abs() < 0.01, "beta({a},{b}): {m}");
        }
    }
}
