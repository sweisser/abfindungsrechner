//! Year-by-year simulation of both scenarios: A = stay, B = take the severance.

use serde::Serialize;

use crate::income::{alg_monthly, year_cash, Seg, YearRow};
use crate::params::Params;
use crate::portfolio::Port;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sim {
    pub a: Vec<YearRow>,
    pub b: Vec<YearRow>,
    pub alg_monthly: f64,
    pub n: u32,
    pub deflator: f64,
    /// Price level at the end of each year relative to the start; index 0 = start (1.0), last = `deflator`.
    pub price_levels: Vec<f64>,
    pub final_a: f64,
    pub final_b: f64,
    pub diff: f64,
    pub real_a: f64,
    pub real_b: f64,
    pub real_diff: f64,
    pub sev_year: u32,
    pub sev_tax: f64,
    pub sev_net: f64,
}

/// Market conditions per year; index `k - 1` holds year `k`.
#[derive(Clone, Debug, PartialEq)]
pub struct MarketPath {
    /// Gross portfolio return (before costs).
    pub returns: Vec<f64>,
    pub inflation: Vec<f64>,
}

impl MarketPath {
    /// The expected values from `p`, repeated for every year of the horizon.
    pub fn constant(p: &Params) -> Self {
        let n = horizon_years(p) as usize;
        MarketPath { returns: vec![p.ret; n], inflation: vec![p.inflation; n] }
    }
}

/// Number of simulated years.
pub(crate) fn horizon_years(p: &Params) -> u32 {
    p.horizon.round().max(0.0) as u32
}

/// Length of the overlap of the intervals [a, b) and [c, d).
fn overlap(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (b.min(d) - a.max(c)).max(0.0)
}

/// Simulation with constant market returns and inflation.
pub fn simulate(p: &Params) -> Sim {
    simulate_path(p, &MarketPath::constant(p))
}

/// Simulation along a given market path; both scenarios see the same market.
pub fn simulate_path(p: &Params, market: &MarketPath) -> Sim {
    let n = horizon_years(p);
    assert!(
        market.returns.len() >= n as usize && market.inflation.len() >= n as usize,
        "market path shorter than the horizon"
    );
    let e = p.exit_month.round();
    let j = p.search_months.round();
    let a = p.alg_months.round().min(j);
    let pay = e + p.pay_delay.round();
    let ky = (pay / 12.0).floor() as u32 + 1;
    let pay_frac = (pay % 12.0) / 12.0;
    let alg_m = alg_monthly(p);

    let (mut sa, mut sb) = (Port::new(p), Port::new(p));
    let (mut rows_a, mut rows_b) = (Vec::new(), Vec::new());
    let mut price_level = 1.0; // at the start of year k
    let mut price_levels = vec![price_level];

    for k in 1..=n {
        let (y0, y1) = (12.0 * (k - 1) as f64, 12.0 * k as f64);
        let ret = market.returns[k as usize - 1];
        let r = ret - p.ter;
        let spend = p.spending * price_level;
        let stay_m = p.salary * (1.0 + p.stay_growth).powi(k as i32 - 1) / 12.0;
        let new_m = p.new_salary * (1.0 + p.new_growth).powi(k as i32 - 1) / 12.0;

        // A: stay
        let mut ca = year_cash(p, &[Seg { m: stay_m, n: 12.0 }], 0.0, 0.0);
        sa.grow_year(p, ret);
        sa.flow(p, (ca.net - spend) * (1.0 + r).powf(0.5), ca.net - spend);
        ca.year = k;
        ca.spend = spend;
        ca.wealth = sa.w;
        rows_a.push(ca);

        // B: take the severance
        let segs = [
            Seg { m: stay_m, n: overlap(0.0, e, y0, y1) },
            Seg { m: new_m, n: if p.new_salary > 0.0 { overlap(e + j, f64::INFINITY, y0, y1) } else { 0.0 } },
        ];
        let alg_mo = overlap(e, e + a, y0, y1);
        let sev = if k == ky { p.severance } else { 0.0 };
        let mut cb = year_cash(p, &segs, alg_mo * alg_m, sev);
        let sev_net = sev - cb.tax_on_sev;
        let rest = cb.net - sev + cb.tax_on_sev - spend;
        sb.grow_year(p, ret);
        sb.flow(p, sev_net * (1.0 + r).powf(1.0 - pay_frac), sev_net);
        sb.flow(p, rest * (1.0 + r).powf(0.5), rest);
        cb.year = k;
        cb.spend = spend;
        cb.wealth = sb.w;
        cb.sev_net = sev_net;
        rows_b.push(cb);
        price_level *= 1.0 + market.inflation[k as usize - 1];
        price_levels.push(price_level);
    }

    let defl = price_level;
    let (fa, fb) = (sa.liquidate(p), sb.liquidate(p));
    let sev_row = rows_b.get((ky.min(n) as usize).wrapping_sub(1));
    Sim {
        alg_monthly: alg_m,
        n,
        deflator: defl,
        price_levels,
        final_a: fa,
        final_b: fb,
        diff: fb - fa,
        real_a: fa / defl,
        real_b: fb / defl,
        real_diff: (fb - fa) / defl,
        sev_year: ky,
        sev_tax: sev_row.map_or(0.0, |r| r.tax_on_sev),
        sev_net: sev_row.map_or(0.0, |r| r.sev_net),
        a: rows_a,
        b: rows_b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;

    #[test]
    fn identical_scenarios_have_zero_difference() {
        let mut p = test_defaults();
        p.severance = 0.0;
        p.new_salary = 90000.0;
        p.search_months = 0.0;
        p.alg_months = 0.0;
        assert!(simulate(&p).diff.abs() < 1e-6);
    }

    #[test]
    fn default_case_matches_reference() {
        // Reference values from the original JS implementation.
        let s = simulate(&test_defaults());
        assert!((s.final_a - 113127.0).abs() < 1.0, "{}", s.final_a);
        assert!((s.final_b - 110892.0).abs() < 1.0, "{}", s.final_b);
        assert!((s.sev_tax - 31222.989).abs() < 0.01);
    }

    fn path(returns: Vec<f64>, inflation: Vec<f64>) -> MarketPath {
        MarketPath { returns, inflation }
    }

    #[test]
    fn constant_path_repeats_the_expected_values() {
        let p = test_defaults();
        let m = MarketPath::constant(&p);
        assert_eq!(m.returns, vec![0.06; 10]);
        assert_eq!(m.inflation, vec![0.02; 10]);
    }

    #[test]
    fn constant_path_reproduces_deterministic_simulation() {
        let p = test_defaults();
        let (s, t) = (simulate(&p), simulate_path(&p, &MarketPath::constant(&p)));
        assert_eq!(s.final_a, t.final_a);
        assert_eq!(s.final_b, t.final_b);
        assert_eq!(s.real_diff, t.real_diff);
    }

    #[test]
    fn each_years_return_is_applied() {
        let mut p = test_defaults();
        p.start_wealth = 100_000.0;
        let base = simulate_path(&p, &path(vec![0.0; 10], vec![0.0; 10]));
        for k in 0..10 {
            let mut r = vec![0.0; 10];
            r[k] = 0.10;
            let s = simulate_path(&p, &path(r, vec![0.0; 10]));
            assert!(s.final_a > base.final_a, "return of year {} ignored", k + 1);
        }
    }

    #[test]
    fn crash_hurts_both_scenarios() {
        let mut p = test_defaults();
        p.start_wealth = 100_000.0;
        let calm = simulate_path(&p, &MarketPath::constant(&p));
        let mut r = vec![0.06; 10];
        r[0] = -0.4;
        let crash = simulate_path(&p, &path(r, vec![0.02; 10]));
        assert!(crash.final_a < calm.final_a);
        assert!(crash.final_b < calm.final_b);
    }

    #[test]
    fn inflation_path_drives_spending_and_deflator() {
        let p = test_defaults();
        let mut infl = vec![0.0; 10];
        infl[0] = 0.10;
        let s = simulate_path(&p, &path(vec![0.06; 10], infl));
        assert_eq!(s.a[0].spend, p.spending);
        assert!((s.a[1].spend - 1.1 * p.spending).abs() < 1e-9);
        assert!((s.a[9].spend - 1.1 * p.spending).abs() < 1e-9);
        assert!((s.deflator - 1.1).abs() < 1e-12);
        assert!((s.real_a - s.final_a / 1.1).abs() < 1e-9);
    }

    #[test]
    fn price_levels_run_from_one_to_the_deflator() {
        let p = test_defaults();
        let infl = vec![0.10, 0.0, 0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let s = simulate_path(&p, &path(vec![0.06; 10], infl));
        assert_eq!(s.price_levels.len(), 11);
        assert_eq!(s.price_levels[0], 1.0);
        assert!((s.price_levels[1] - 1.1).abs() < 1e-12);
        assert!((s.price_levels[3] - 1.1 * 1.05).abs() < 1e-12);
        assert_eq!(s.price_levels[10], s.deflator);
    }
}
