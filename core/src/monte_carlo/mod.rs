//! Monte Carlo analysis: run the simulation many times with uncertain inputs.

mod market;
mod paths;
mod stats;
mod uncertainty;

pub use market::MarketModel;
pub use paths::{PathStats, WealthPaths};
pub use stats::{Histogram, Percentiles, Summary};
pub use uncertainty::Uncertainty;

use serde::Serialize;

use crate::params::Params;
use crate::random::Rng;
use crate::simulation::simulate_path;
use paths::WealthCollector;
use stats::summarize;

const HISTOGRAM_BINS: usize = 30;
/// Single runs kept per scenario for drawing; enough to show the spread without hiding the bands.
const SAMPLE_PATHS: usize = 40;

/// Final outcomes in today's money (wealth after liquidation tax, deflated), plus wealth paths over time.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarlo {
    pub runs: u32,
    pub real_a: Summary,
    pub real_b: Summary,
    /// B minus A; `prob_positive` is the probability that taking the severance wins.
    pub real_diff: Summary,
    pub wealth_a: WealthPaths,
    pub wealth_b: WealthPaths,
}

/// Simulate `runs` (≥ 1) random futures, reproducibly for a given `seed`.
pub fn run(p: &Params, uncertainty: &Uncertainty, runs: u32, seed: u64) -> MonteCarlo {
    let mut rng = Rng::new(seed);
    let (mut a, mut b, mut diff) = (Vec::new(), Vec::new(), Vec::new());
    let (mut wealth_a, mut wealth_b) = (WealthCollector::new(SAMPLE_PATHS), WealthCollector::new(SAMPLE_PATHS));
    for _ in 0..runs.max(1) {
        let (q, market) = uncertainty.sample(p, &mut rng);
        let sim = simulate_path(&q, &market);
        wealth_a.push(q.start_wealth, &sim.a, &sim.price_levels);
        wealth_b.push(q.start_wealth, &sim.b, &sim.price_levels);
        a.push(sim.real_a);
        b.push(sim.real_b);
        diff.push(sim.real_diff);
    }
    MonteCarlo {
        runs: diff.len() as u32,
        real_a: summarize(&a, HISTOGRAM_BINS),
        real_b: summarize(&b, HISTOGRAM_BINS),
        real_diff: summarize(&diff, HISTOGRAM_BINS),
        wealth_a: wealth_a.finish(),
        wealth_b: wealth_b.finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;
    use crate::random::Pert;
    use crate::simulation::simulate;

    fn uncertain() -> Uncertainty {
        Uncertainty {
            search_months: Some(Pert::new(1.0, 6.0, 24.0).unwrap()),
            new_salary: Some(Pert::new(55_000.0, 80_000.0, 100_000.0).unwrap()),
            market: MarketModel { return_vol: 0.16, inflation_vol: 0.01 },
            ..Default::default()
        }
    }

    #[test]
    fn reports_the_number_of_runs() {
        assert_eq!(run(&test_defaults(), &uncertain(), 123, 1).runs, 123);
    }

    #[test]
    fn same_seed_reproduces_results() {
        let p = test_defaults();
        let (a, b, c) = (run(&p, &uncertain(), 300, 7), run(&p, &uncertain(), 300, 7), run(&p, &uncertain(), 300, 8));
        assert_eq!(a.real_diff.percentiles, b.real_diff.percentiles);
        assert_ne!(a.real_diff.percentiles, c.real_diff.percentiles);
    }

    #[test]
    fn without_uncertainty_every_run_is_the_deterministic_result() {
        let p = test_defaults();
        let mc = run(&p, &Uncertainty::default(), 50, 1);
        let expected = simulate(&p).real_diff;
        assert_eq!(mc.real_diff.percentiles.p5, expected);
        assert_eq!(mc.real_diff.percentiles.p95, expected);
        assert_eq!(mc.real_diff.prob_positive, if expected > 0.0 { 1.0 } else { 0.0 });
    }

    #[test]
    fn both_scenarios_share_the_market_path() {
        // Identical scenarios: any market randomness must cancel in B - A.
        let mut p = test_defaults();
        p.severance = 0.0;
        p.new_salary = p.salary;
        p.search_months = 0.0;
        p.alg_months = 0.0;
        let u = Uncertainty { market: MarketModel { return_vol: 0.2, inflation_vol: 0.02 }, ..Default::default() };
        let mc = run(&p, &u, 200, 3);
        assert!(mc.real_a.percentiles.p95 > mc.real_a.percentiles.p5 + 1000.0, "market not random");
        assert!(mc.real_diff.percentiles.p5.abs() < 1e-6);
        assert!(mc.real_diff.percentiles.p95.abs() < 1e-6);
    }

    #[test]
    fn wealth_paths_cover_start_and_every_year() {
        let p = test_defaults();
        let mc = run(&p, &uncertain(), 100, 1);
        for w in [&mc.wealth_a, &mc.wealth_b] {
            for s in [&w.nominal, &w.real] {
                assert_eq!(s.percentiles.len(), 11);
                assert_eq!(s.percentiles[0].p5, p.start_wealth);
                assert_eq!(s.samples.len(), SAMPLE_PATHS);
                assert!(s.samples.iter().all(|x| x.len() == 11));
                let last = &s.percentiles[10];
                assert!(last.p5 < last.p25 && last.p25 < last.p50 && last.p50 < last.p75 && last.p75 < last.p95);
            }
        }
    }

    #[test]
    fn sample_paths_are_limited_by_the_number_of_runs() {
        let mc = run(&test_defaults(), &uncertain(), 3, 1);
        assert_eq!(mc.wealth_a.real.samples.len(), 3);
        assert_eq!(mc.wealth_b.nominal.samples.len(), 3);
    }

    #[test]
    fn without_uncertainty_the_paths_are_the_deterministic_path() {
        let p = test_defaults();
        let mc = run(&p, &Uncertainty::default(), 20, 1);
        let sim = simulate(&p);
        for (k, row) in sim.b.iter().enumerate() {
            let (nominal, real) = (&mc.wealth_b.nominal.percentiles[k + 1], &mc.wealth_b.real.percentiles[k + 1]);
            assert_eq!((nominal.p5, nominal.p95), (row.wealth, row.wealth));
            assert_eq!(real.p50, row.wealth / sim.price_levels[k + 1]);
        }
    }

    #[test]
    fn larger_severance_wins_more_often() {
        let mut p = test_defaults();
        let low = run(&p, &uncertain(), 500, 2).real_diff.prob_positive;
        p.severance = 150_000.0;
        let high = run(&p, &uncertain(), 500, 2).real_diff.prob_positive;
        assert!(high > low, "{high} <= {low}");
    }
}
