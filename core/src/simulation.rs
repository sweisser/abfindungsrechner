//! Year-by-year simulation of both scenarios: A = stay, B = take the severance.

use serde::Serialize;

use crate::income::{alg_monthly, year_cash, Seg, YearRow};
use crate::params::Params;
use crate::pension::{alg_points, points, value_per_point};
use crate::portfolio::Port;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sim {
    pub a: Vec<YearRow>,
    pub b: Vec<YearRow>,
    pub alg_monthly: f64,
    /// Number of rows: calendar years touched by the horizon.
    pub n: u32,
    pub deflator: f64,
    /// Price level at the end of each row's year relative to today; index 0 = start (1.0), last = `deflator`.
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
    /// Wealth after selling everything at the end of the horizon, nominal, without pension rights.
    pub liquid_a: f64,
    pub liquid_b: f64,
    /// Entgeltpunkte earned within the horizon.
    pub points_a: f64,
    pub points_b: f64,
    /// Value of these Entgeltpunkte at the end of the horizon, nominal; part of `final_*` if `pension_in_wealth`.
    pub pension_a: f64,
    pub pension_b: f64,
}

/// Market conditions per calendar year of the horizon (see `periods`), as yearly rates.
#[derive(Clone, Debug, PartialEq)]
pub struct MarketPath {
    /// Gross portfolio return (before costs).
    pub returns: Vec<f64>,
    pub inflation: Vec<f64>,
}

impl MarketPath {
    /// The expected values from `p`, repeated for every period of the horizon.
    pub fn constant(p: &Params) -> Self {
        let n = periods(p).len();
        MarketPath { returns: vec![p.ret; n], inflation: vec![p.inflation; n] }
    }
}

/// Number of simulated years.
pub(crate) fn horizon_years(p: &Params) -> u32 {
    p.horizon.round().max(0.0) as u32
}

/// Months of the current calendar year before today.
fn months_before_start(p: &Params) -> f64 {
    p.start_month.round().clamp(1.0, 12.0) - 1.0
}

/// The part of one calendar (= tax) year inside the horizon, in months from today.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Period {
    pub start: f64,
    pub end: f64,
}

impl Period {
    pub fn years(&self) -> f64 {
        (self.end - self.start) / 12.0
    }
}

/// The horizon cut at each new year; the first and last period are partial unless today is in January.
pub(crate) fn periods(p: &Params) -> Vec<Period> {
    let (before, h) = (months_before_start(p), 12.0 * horizon_years(p) as f64);
    (0..)
        .map(|k| Period { start: (12.0 * k as f64 - before).max(0.0), end: (12.0 * (k + 1) as f64 - before).min(h) })
        .take_while(|q| q.start < h)
        .collect()
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
///
/// Each row is one calendar year, taxed as a whole. The months of the first year before today are
/// earned at the current salary in both scenarios: they count for the year's income tax, but their
/// cash (net of the tax withheld on them) is already spent and is left out of the rows.
pub fn simulate_path(p: &Params, market: &MarketPath) -> Sim {
    let periods = periods(p);
    let n = periods.len() as u32;
    assert!(
        market.returns.len() >= periods.len() && market.inflation.len() >= periods.len(),
        "market path shorter than the horizon"
    );
    let before = months_before_start(p);
    let e = p.exit_month.round();
    let j = p.search_months.round();
    let a = p.alg_months.round().min(j);
    let pay = e + p.pay_delay.round();
    let ky = periods.iter().position(|q| q.start <= pay && pay < q.end).map_or(n + 1, |i| i as u32 + 1);
    let alg_m = alg_monthly(p);

    let (mut sa, mut sb) = (Port::new(p), Port::new(p));
    let (mut rows_a, mut rows_b) = (Vec::new(), Vec::new());
    let mut price_level = 1.0; // at the start of the period
    let mut price_levels = vec![price_level];

    for (k, q) in (1..).zip(&periods) {
        let (y0, y1, years) = (q.start, q.end, q.years());
        let ret = market.returns[k as usize - 1];
        let growth = |t: f64| (1.0 + ret - p.ter).powf(t);
        let spend = p.spending * price_level * years;
        // Raises every January.
        let stay_m = p.salary * (1.0 + p.stay_growth).powi(k as i32 - 1) / 12.0;
        let new_m = p.new_salary * (1.0 + p.new_growth).powi(k as i32 - 1) / 12.0;
        let earlier = if k == 1 { before } else { 0.0 };

        // A: stay. The salary is constant within the year, so the months before today had their share withheld.
        let full_a = year_cash(p, &[Seg { m: stay_m, n: earlier + y1 - y0 }], 0.0, 0.0);
        let paid = full_a.scaled(earlier / (earlier + y1 - y0));
        let mut ca = full_a.minus(&paid);
        sa.grow(p, ret, years);
        sa.flow(p, (ca.net - spend) * growth(years / 2.0), ca.net - spend);
        ca.year = k;
        ca.months = y1 - y0;
        ca.points = points(p, stay_m, y1 - y0, k);
        ca.spend = spend;
        ca.wealth = sa.w;
        rows_a.push(ca);

        // B: take the severance
        let segs = [
            Seg { m: stay_m, n: earlier + overlap(0.0, e, y0, y1) },
            Seg { m: new_m, n: if p.new_salary > 0.0 { overlap(e + j, f64::INFINITY, y0, y1) } else { 0.0 } },
        ];
        let alg_mo = overlap(e, e + a, y0, y1);
        let sev = if k == ky { p.severance } else { 0.0 };
        let mut cb = year_cash(p, &segs, alg_mo * alg_m, sev).minus(&paid);
        let sev_net = sev - cb.tax_on_sev;
        let rest = cb.net - sev + cb.tax_on_sev - spend;
        sb.grow(p, ret, years);
        sb.flow(p, sev_net * growth((y1 - pay) / 12.0), sev_net);
        sb.flow(p, rest * growth(years / 2.0), rest);
        cb.year = k;
        cb.months = y1 - y0;
        cb.points =
            points(p, stay_m, overlap(0.0, e, y0, y1), k) + points(p, new_m, segs[1].n, k) + alg_points(p, alg_mo, k);
        cb.spend = spend;
        cb.wealth = sb.w;
        cb.sev_net = sev_net;
        rows_b.push(cb);
        price_level *= (1.0 + market.inflation[k as usize - 1]).powf(years);
        price_levels.push(price_level);
    }

    let defl = price_level;
    let (points_a, points_b) =
        (rows_a.iter().map(|r| r.points).sum::<f64>(), rows_b.iter().map(|r| r.points).sum::<f64>());
    let per_point = value_per_point(p, horizon_years(p) as f64);
    let (pension_a, pension_b) = (points_a * per_point, points_b * per_point);
    let counted = if p.pension_in_wealth { 1.0 } else { 0.0 };
    let (liquid_a, liquid_b) = (sa.liquidate(p), sb.liquidate(p));
    let (fa, fb) = (liquid_a + counted * pension_a, liquid_b + counted * pension_b);
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
        liquid_a,
        liquid_b,
        points_a,
        points_b,
        pension_a,
        pension_b,
        a: rows_a,
        b: rows_b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;
    use crate::pension::value_per_point;

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

    fn starting_in(month: f64) -> Params {
        let mut p = test_defaults();
        p.start_month = month;
        p
    }

    fn bounds(p: &Params) -> Vec<(f64, f64)> {
        periods(p).iter().map(|q| (q.start, q.end)).collect()
    }

    #[test]
    fn periods_split_the_horizon_at_new_year() {
        let mut p = starting_in(10.0);
        p.horizon = 2.0;
        assert_eq!(bounds(&p), vec![(0.0, 3.0), (3.0, 15.0), (15.0, 24.0)]);
        p.start_month = 1.0;
        assert_eq!(bounds(&p), vec![(0.0, 12.0), (12.0, 24.0)]);
    }

    #[test]
    fn start_month_is_clamped_to_the_calendar() {
        assert_eq!(bounds(&starting_in(0.0)), bounds(&starting_in(1.0)));
        assert_eq!(bounds(&starting_in(15.0)), bounds(&starting_in(12.0)));
    }

    #[test]
    fn growth_and_inflation_do_not_depend_on_the_start_month() {
        for month in [1.0, 4.0, 10.0] {
            let mut p = starting_in(month);
            (p.salary, p.new_salary, p.severance, p.spending) = (0.0, 0.0, 0.0, 0.0);
            p.start_wealth = 100_000.0;
            let s = simulate(&p);
            let w = s.a.last().unwrap().wealth;
            assert!((w - 100_000.0 * 1.058f64.powi(10)).abs() < 1e-6, "{month}: {w}");
            assert!((s.deflator - 1.02f64.powi(10)).abs() < 1e-12);
            assert_eq!(s.price_levels.len(), s.a.len() + 1);
        }
    }

    #[test]
    fn spending_is_prorated_to_the_months_of_the_year() {
        let s = simulate(&starting_in(10.0));
        assert_eq!(s.a.len(), 11);
        assert_eq!((s.a[0].months, s.a[1].months, s.a[10].months), (3.0, 12.0, 9.0));
        assert!((s.a[0].spend - 45000.0 * 0.25).abs() < 1e-9);
        assert!((s.a[1].spend - 45000.0 * 1.02f64.powf(0.25)).abs() < 1e-6);
    }

    #[test]
    fn months_before_today_count_for_tax_but_not_for_cash() {
        // Staying, the salary is constant within the year, so its tax is withheld evenly.
        let (full, part) = (simulate(&starting_in(1.0)), simulate(&starting_in(10.0)));
        for (x, y) in [(full.a[0].wage, part.a[0].wage), (full.a[0].tax, part.a[0].tax), (full.a[0].net, part.a[0].net)]
        {
            assert!((y - 0.25 * x).abs() < 1e-6, "{x} {y}");
        }
    }

    #[test]
    fn months_before_today_raise_the_tax_on_the_severance() {
        // Leaving in October: the same calendar year whether today is January or October.
        let mut jan = starting_in(1.0);
        jan.exit_month = 9.0;
        let oct = starting_in(10.0);
        assert!((simulate(&jan).sev_tax - simulate(&oct).sev_tax).abs() < 1e-6);
    }

    #[test]
    fn severance_paid_in_january_is_taxed_in_the_new_year() {
        let mut p = starting_in(10.0);
        p.new_salary = 0.0;
        p.pay_delay = 2.0; // December
        let dec = simulate(&p);
        p.pay_delay = 3.0; // January
        let jan = simulate(&p);
        assert_eq!((dec.sev_year, jan.sev_year), (1, 2));
        assert!(jan.sev_tax < dec.sev_tax - 5000.0, "{} vs {}", jan.sev_tax, dec.sev_tax);
    }

    const AVG: f64 = 51944.0;

    #[test]
    fn staying_earns_points_on_the_salary() {
        let s = simulate(&test_defaults());
        assert!((s.a[0].points - 90000.0 / AVG).abs() < 1e-9);
        // Year 2: salary +2 %, average earnings +2.5 %.
        assert!((s.a[1].points - 90000.0 * 1.02 / (AVG * 1.025)).abs() < 1e-9);
        assert!((s.points_a - s.a.iter().map(|r| r.points).sum::<f64>()).abs() < 1e-12);
    }

    #[test]
    fn leaving_earns_points_from_alg_and_the_new_job() {
        // Leave now, 6 months on ALG I (80 % of the old salary counts), then the new job.
        let s = simulate(&test_defaults());
        let expected = (6.0 * 0.8 * 90000.0 + 6.0 * 80000.0) / 12.0 / AVG;
        assert!((s.b[0].points - expected).abs() < 1e-9);
        assert!((s.points_b - s.b.iter().map(|r| r.points).sum::<f64>()).abs() < 1e-12);
    }

    #[test]
    fn search_months_without_alg_earn_no_points() {
        let mut p = test_defaults();
        p.alg_months = 0.0;
        let s = simulate(&p);
        assert!((s.b[0].points - 6.0 * 80000.0 / 12.0 / AVG).abs() < 1e-9);
    }

    #[test]
    fn months_before_today_earn_no_points_here() {
        let s = simulate(&starting_in(10.0));
        assert!((s.a[0].points - 3.0 * 90000.0 / 12.0 / AVG).abs() < 1e-9);
    }

    #[test]
    fn pension_rights_enter_final_wealth_when_switched_on() {
        let mut p = test_defaults();
        let off = simulate(&p);
        p.pension_in_wealth = true;
        let on = simulate(&p);
        let v = value_per_point(&p, 10.0);
        assert!((off.pension_a - off.points_a * v).abs() < 1e-6, "value reported either way");
        assert!((off.pension_b - off.points_b * v).abs() < 1e-6);
        assert!((on.final_a - (off.final_a + off.pension_a)).abs() < 1e-6);
        assert!((on.final_b - (off.final_b + off.pension_b)).abs() < 1e-6);
        assert!((on.real_diff - on.diff / on.deflator).abs() < 1e-6);
        assert!(on.diff < off.diff, "B earns fewer points");
        assert_eq!((on.liquid_a, on.liquid_b), (off.final_a, off.final_b));
    }

    #[test]
    fn identical_scenarios_have_equal_pensions() {
        let mut p = test_defaults();
        (p.severance, p.new_salary, p.search_months, p.alg_months) = (0.0, 90000.0, 0.0, 0.0);
        p.pension_in_wealth = true;
        let s = simulate(&p);
        assert!((s.points_a - s.points_b).abs() < 1e-12);
        assert!(s.diff.abs() < 1e-6);
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
