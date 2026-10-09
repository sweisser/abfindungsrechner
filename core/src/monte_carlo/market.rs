//! Random market paths: yearly portfolio returns and inflation.

use serde::Deserialize;

use crate::params::Params;
use crate::random::Rng;
use crate::simulation::{horizon_years, MarketPath};

/// How strongly inflation sticks to last year's deviation from its mean (AR(1) coefficient).
const INFLATION_PERSISTENCE: f64 = 0.7;

/// Volatilities around the expected values in `Params` (`ret`, `inflation`).
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MarketModel {
    /// Standard deviation of the yearly gross return.
    pub return_vol: f64,
    /// Long-run standard deviation of yearly inflation.
    pub inflation_vol: f64,
}

impl MarketModel {
    /// Returns are lognormal (mean `p.ret`, sd `return_vol`), independent between years.
    /// Inflation mean-reverts to `p.inflation`, starting there.
    pub fn sample_path(&self, p: &Params, rng: &mut Rng) -> MarketPath {
        let n = horizon_years(p) as usize;
        let shock_sd = self.inflation_vol * (1.0 - INFLATION_PERSISTENCE.powi(2)).sqrt();
        let mut returns = Vec::with_capacity(n);
        let mut inflation = Vec::with_capacity(n);
        let mut deviation = 0.0; // of inflation from its mean
        for year in 0..n {
            // Draw both shocks every year, so the random stream does not depend on the settings.
            let (z_ret, z_infl) = (rng.normal(), rng.normal());
            returns.push(lognormal_return(p.ret, self.return_vol, z_ret));
            if year > 0 {
                deviation = INFLATION_PERSISTENCE * deviation + shock_sd * z_infl;
            }
            inflation.push(p.inflation + deviation);
        }
        MarketPath { returns, inflation }
    }
}

/// Return whose growth factor 1 + r is lognormal with the given mean and sd of r.
fn lognormal_return(mean: f64, vol: f64, z: f64) -> f64 {
    if vol == 0.0 {
        return mean;
    }
    let var_log = (1.0 + (vol / (1.0 + mean)).powi(2)).ln();
    let mu_log = (1.0 + mean).ln() - var_log / 2.0;
    (mu_log + var_log.sqrt() * z).exp() - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;

    const PATHS: usize = 4000;

    fn pooled(m: MarketModel, pick: fn(&MarketPath) -> &Vec<f64>) -> Vec<Vec<f64>> {
        let p = test_defaults();
        let mut rng = Rng::new(11);
        (0..PATHS).map(|_| pick(&m.sample_path(&p, &mut rng)).clone()).collect()
    }

    fn mean_sd(xs: &[f64]) -> (f64, f64) {
        let m = xs.iter().sum::<f64>() / xs.len() as f64;
        let v = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64;
        (m, v.sqrt())
    }

    #[test]
    fn without_volatility_the_path_is_constant() {
        let p = test_defaults();
        let path = MarketModel::default().sample_path(&p, &mut Rng::new(1));
        assert_eq!(path, MarketPath::constant(&p));
    }

    #[test]
    fn path_covers_the_horizon() {
        let mut p = test_defaults();
        p.horizon = 7.0;
        let m = MarketModel { return_vol: 0.15, inflation_vol: 0.01 };
        let path = m.sample_path(&p, &mut Rng::new(1));
        assert_eq!((path.returns.len(), path.inflation.len()), (7, 7));
    }

    #[test]
    fn returns_have_the_expected_mean_and_volatility() {
        let m = MarketModel { return_vol: 0.16, inflation_vol: 0.0 };
        let all: Vec<f64> = pooled(m, |x| &x.returns).concat();
        assert!(all.iter().all(|&r| r > -1.0), "a return lost more than everything");
        let (mean, sd) = mean_sd(&all);
        assert!((mean - 0.06).abs() < 0.003, "{mean}");
        assert!((sd - 0.16).abs() < 0.003, "{sd}");
    }

    #[test]
    fn inflation_varies_around_its_mean_and_is_persistent() {
        let m = MarketModel { return_vol: 0.0, inflation_vol: 0.015 };
        let paths = pooled(m, |x| &x.inflation);
        // Year 1 starts at the mean; later years have (nearly) the long-run spread.
        let late: Vec<f64> = paths.iter().map(|x| x[9]).collect();
        let (mean, sd) = mean_sd(&late);
        assert!((mean - 0.02).abs() < 0.001, "{mean}");
        assert!((sd - 0.015).abs() < 0.0015, "{sd}");
        // Lag-1 correlation between consecutive years ≈ persistence.
        let (x, y): (Vec<f64>, Vec<f64>) = paths.iter().map(|x| (x[8] - 0.02, x[9] - 0.02)).unzip();
        let corr = x.iter().zip(&y).map(|(a, b)| a * b).sum::<f64>()
            / (x.iter().map(|a| a * a).sum::<f64>() * y.iter().map(|b| b * b).sum::<f64>()).sqrt();
        assert!((corr - INFLATION_PERSISTENCE).abs() < 0.05, "{corr}");
    }
}
