//! Which inputs are uncertain, and how: one scenario = sampled parameters + a market path.

use serde::Deserialize;

use super::MarketModel;
use crate::params::Params;
use crate::random::{Pert, Rng};
use crate::simulation::MarketPath;

/// Distributions for uncertain inputs; `None` keeps the value from `Params`.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Uncertainty {
    pub search_months: Option<Pert>,
    pub new_salary: Option<Pert>,
    pub stay_growth: Option<Pert>,
    pub new_growth: Option<Pert>,
    pub market: MarketModel,
}

impl Uncertainty {
    /// Draw one possible future: personal inputs plus a market path shared by both scenarios.
    pub fn sample(&self, p: &Params, rng: &mut Rng) -> (Params, MarketPath) {
        let mut q = p.clone();
        draw(&mut q.search_months, self.search_months, rng);
        draw(&mut q.new_salary, self.new_salary, rng);
        draw(&mut q.stay_growth, self.stay_growth, rng);
        draw(&mut q.new_growth, self.new_growth, rng);
        let path = self.market.sample_path(&q, rng);
        (q, path)
    }
}

fn draw(value: &mut f64, dist: Option<Pert>, rng: &mut Rng) {
    if let Some(d) = dist {
        *value = d.sample(rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;

    fn pert(min: f64, mode: f64, max: f64) -> Option<Pert> {
        Some(Pert::new(min, mode, max).unwrap())
    }

    #[test]
    fn empty_json_means_nothing_is_uncertain() {
        let u: Uncertainty = serde_json::from_str("{}").unwrap();
        let p = test_defaults();
        let (q, path) = u.sample(&p, &mut Rng::new(1));
        assert_eq!(format!("{q:?}"), format!("{p:?}"));
        assert_eq!(path, MarketPath::constant(&p));
    }

    #[test]
    fn uncertain_inputs_are_drawn_from_their_ranges() {
        let u = Uncertainty {
            search_months: pert(2.0, 6.0, 24.0),
            new_salary: pert(60_000.0, 80_000.0, 95_000.0),
            stay_growth: pert(0.0, 0.02, 0.04),
            new_growth: pert(0.01, 0.02, 0.05),
            market: MarketModel::default(),
        };
        let p = test_defaults();
        let mut rng = Rng::new(5);
        let mut seen_different = false;
        for _ in 0..200 {
            let (q, _) = u.sample(&p, &mut rng);
            assert!((2.0..=24.0).contains(&q.search_months));
            assert!((60_000.0..=95_000.0).contains(&q.new_salary));
            assert!((0.0..=0.04).contains(&q.stay_growth));
            assert!((0.01..=0.05).contains(&q.new_growth));
            seen_different |= q.new_salary != p.new_salary;
        }
        assert!(seen_different);
    }

    #[test]
    fn certain_inputs_stay_untouched() {
        let u = Uncertainty { new_salary: pert(1.0, 2.0, 3.0), ..Default::default() };
        let p = test_defaults();
        let (q, _) = u.sample(&p, &mut Rng::new(2));
        assert_eq!(q.search_months, p.search_months);
        assert_eq!(q.severance, p.severance);
    }

    #[test]
    fn market_volatility_produces_a_random_path() {
        let u = Uncertainty { market: MarketModel { return_vol: 0.15, inflation_vol: 0.0 }, ..Default::default() };
        let p = test_defaults();
        let (_, path) = u.sample(&p, &mut Rng::new(3));
        assert_ne!(path.returns, MarketPath::constant(&p).returns);
    }

    #[test]
    fn parses_camel_case_json() {
        let u: Uncertainty = serde_json::from_str(
            r#"{"searchMonths":{"min":1,"mode":3,"max":12},"market":{"returnVol":0.15,"inflationVol":0.01}}"#,
        )
        .unwrap();
        assert_eq!(u.search_months, pert(1.0, 3.0, 12.0));
        assert_eq!(u.market.return_vol, 0.15);
    }
}
