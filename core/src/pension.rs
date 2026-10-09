//! Statutory pension (gesetzliche Rentenversicherung): Entgeltpunkte earned and their value.

use std::cell::Cell;

use crate::params::Params;

/// The year of the 2026 constants (Rentenwert, Durchschnittsentgelt, ceilings); year 1 of the simulation.
const THIS_YEAR: f64 = 2026.0;
/// Gompertz slope: adult mortality doubles about every 7 years of age.
const GOMPERTZ_SLOPE: f64 = std::f64::consts::LN_2 / 7.0;
/// Nobody is assumed to live longer than this.
const MAX_AGE: f64 = 120.0;

/// Wage index of calendar year `k` (1 = this year): average earnings, Rentenwert and ceiling grow each January.
fn index(p: &Params, k: u32) -> f64 {
    (1.0 + p.pension_growth).powi(k as i32 - 1)
}

/// Entgeltpunkte for `months` of the monthly gross `m` in calendar year `k`, capped at the contribution ceiling.
pub fn points(p: &Params, m: f64, months: f64, k: u32) -> f64 {
    let i = index(p, k);
    months * m.min(p.bbg_rv / 12.0 * i) / (p.avg_earnings * i)
}

/// Entgeltpunkte for `months` on ALG I: the Agentur für Arbeit pays contributions on 80 % of the
/// gross salary the benefit is based on (§166 SGB VI).
pub fn alg_points(p: &Params, months: f64, k: u32) -> f64 {
    points(p, 0.8 * p.salary.min(p.bbg_rv) / 12.0, months, k)
}

/// Share of a pension starting in `year` that is subject to income tax (§22 EStG):
/// 82.5 % for 2023, rising by 0.5 points a year to 100 % from 2058.
pub fn taxable_share(year: f64) -> f64 {
    (0.825 + 0.005 * (year - 2023.0)).clamp(0.0, 1.0)
}

/// Net pension per euro of gross pension: minus the retiree's health and care contributions
/// (half of 14.6 % + half the Zusatzbeitrag, the full care rate) and the income tax on the taxable
/// share, where the contributions are deductible.
pub fn net_share(p: &Params, start_year: f64) -> f64 {
    let contributions = 0.073 + p.zusatz / 2.0 + p.pv_rate + 0.018;
    1.0 - contributions - p.pension_tax * (taxable_share(start_year) - contributions).max(0.0)
}

/// Gompertz mortality: force of mortality `b · e^(c · age)`.
pub struct Mortality {
    b: f64,
}

impl Mortality {
    /// Calibrated so that the remaining life expectancy at `age` is `years`.
    pub fn calibrated(age: f64, years: f64) -> Self {
        // Every simulation needs this, with the same inputs: remember the last result.
        thread_local!(static LAST: Cell<Option<(f64, f64, f64)>> = const { Cell::new(None) });
        if let Some((a, y, b)) = LAST.get() {
            if (a, y) == (age, years) {
                return Mortality { b };
            }
        }
        // Life expectancy falls as `b` rises: bisect on ln b.
        let (mut lo, mut hi) = (1e-12f64.ln(), 0.0);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if (Mortality { b: mid.exp() }).life_expectancy(age) > years {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let b = ((lo + hi) / 2.0).exp();
        LAST.set(Some((age, years, b)));
        Mortality { b }
    }

    /// Probability that someone aged `age` is still alive `t` years later.
    pub fn survival(&self, age: f64, t: f64) -> f64 {
        let c = GOMPERTZ_SLOPE;
        (-self.b / c * (c * age).exp() * ((c * t).exp() - 1.0)).exp()
    }

    /// Remaining life expectancy at `age`.
    pub fn life_expectancy(&self, age: f64) -> f64 {
        months_left(age).map(|m| self.survival(age, (m + 0.5) / 12.0)).sum::<f64>() / 12.0
    }
}

/// Month indices from `age` up to the maximum age.
fn months_left(age: f64) -> impl Iterator<Item = f64> {
    (0..(12.0 * (MAX_AGE - age)).max(0.0).ceil() as u32).map(f64::from)
}

/// Value of one Entgeltpunkt at the end of the horizon, `years` from today, in nominal euros.
///
/// The pension is paid monthly from the retirement age (or the end of the horizon, if later) for life.
/// It grows with `pension_growth`, is net of contributions and tax, is weighted by the chance to be alive
/// (given alive at the end of the horizon) and is discounted to the end of the horizon at `pension_discount`.
pub fn value_per_point(p: &Params, years: f64) -> f64 {
    // The Monte Carlo varies none of these inputs: remember the last result.
    type Key = [f64; 10];
    thread_local!(static LAST: Cell<Option<(Key, f64)>> = const { Cell::new(None) });
    let key: Key = [
        years,
        p.age,
        p.retirement_age,
        p.life_expectancy,
        p.rentenwert,
        p.pension_growth,
        p.pension_discount,
        p.pension_tax,
        p.zusatz,
        p.pv_rate,
    ];
    if let Some((k, v)) = LAST.get() {
        if k == key {
            return v;
        }
    }
    let v = compute_value_per_point(p, years);
    LAST.set(Some((key, v)));
    v
}

fn compute_value_per_point(p: &Params, years: f64) -> f64 {
    let mortality = Mortality::calibrated(p.retirement_age, p.life_expectancy);
    let age_end = p.age + years;
    // Years from the end of the horizon to the first payment.
    let wait = (p.retirement_age - age_end).max(0.0);
    // The taxable share is fixed by the year the pension starts.
    let net = net_share(p, THIS_YEAR + (p.retirement_age - p.age));
    // Payment in the middle of each month; growth and discounting advance by one month per step.
    let t0 = wait + 0.5 / 12.0;
    let mut amount = p.rentenwert
        * net
        * ((1.0 + p.pension_growth) / (1.0 + p.pension_discount)).powf(t0)
        * (1.0 + p.pension_growth).powf(years);
    let step = ((1.0 + p.pension_growth) / (1.0 + p.pension_discount)).powf(1.0 / 12.0);
    let mut total = 0.0;
    for m in months_left(age_end + wait) {
        total += mortality.survival(age_end, t0 + m / 12.0) * amount;
        amount *= step;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1.0)
    }

    #[test]
    fn salary_at_average_earnings_earns_one_point_a_year() {
        let p = test_defaults();
        assert!(close(points(&p, 51944.0 / 12.0, 12.0, 1), 1.0, 1e-12));
        assert!(close(points(&p, 51944.0 / 12.0, 6.0, 1), 0.5, 1e-12));
        // Average earnings grow each January.
        assert!(close(points(&p, 51944.0 * 1.025f64.powi(2) / 12.0, 12.0, 3), 1.0, 1e-12));
    }

    #[test]
    fn salary_above_the_ceiling_is_capped() {
        let p = test_defaults();
        assert!(close(points(&p, 200_000.0 / 12.0, 12.0, 1), 101_400.0 / 51944.0, 1e-12));
        assert!(close(points(&p, 200_000.0 / 12.0, 12.0, 4), 101_400.0 / 51944.0, 1e-12));
    }

    #[test]
    fn alg_months_earn_on_eighty_percent_of_the_old_salary() {
        let mut p = test_defaults();
        assert!(close(alg_points(&p, 12.0, 1), 0.8 * 90_000.0 / 51944.0, 1e-12));
        p.salary = 300_000.0;
        assert!(close(alg_points(&p, 12.0, 1), 0.8 * 101_400.0 / 51944.0, 1e-12));
    }

    #[test]
    fn taxable_share_rises_to_full_by_2058() {
        assert!(close(taxable_share(2026.0), 0.84, 1e-12));
        assert!(close(taxable_share(2041.0), 0.915, 1e-12));
        assert_eq!(taxable_share(2058.0), 1.0);
        assert_eq!(taxable_share(2070.0), 1.0);
    }

    #[test]
    fn net_share_takes_off_contributions_and_tax() {
        let p = test_defaults();
        // Contributions 7.3 + 1.45 + 2.4 + 1.8 = 12.95 %; fully taxable from 2058.
        assert!(close(net_share(&p, 2060.0), 1.0 - 0.1295 - 0.25 * (1.0 - 0.1295), 1e-12));
    }

    #[test]
    fn mortality_matches_the_life_expectancy() {
        for (age, e) in [(67.0, 20.0), (67.0, 17.0), (63.0, 23.0)] {
            let m = Mortality::calibrated(age, e);
            assert!((m.life_expectancy(age) - e).abs() < 0.01, "{age} {e}: {}", m.life_expectancy(age));
            // Ten years later about 6.5–8 years less, as in the Destatis life tables.
            let later = m.life_expectancy(age + 10.0);
            assert!(later > e - 8.0 && later < e - 6.5, "{later}");
        }
    }

    #[test]
    fn survival_falls_with_time() {
        let m = Mortality::calibrated(67.0, 20.0);
        assert_eq!(m.survival(67.0, 0.0), 1.0);
        let s: Vec<f64> = [5.0, 10.0, 20.0, 30.0].iter().map(|&t| m.survival(67.0, t)).collect();
        assert!(s.windows(2).all(|w| w[1] < w[0]), "{s:?}");
        assert!(s[0] > 0.9 && s[3] < 0.15, "{s:?}");
    }

    /// Retiring right at the end of the horizon, with discount = growth: the payments keep their
    /// value at the end of the horizon, so a point is worth about life expectancy × yearly net pension.
    fn retiring_at_horizon_end() -> Params {
        let mut p = test_defaults();
        (p.age, p.retirement_age) = (57.0, 67.0);
        p.pension_discount = p.pension_growth;
        p
    }

    #[test]
    fn a_point_is_worth_life_expectancy_times_the_yearly_net_pension() {
        let p = retiring_at_horizon_end();
        let yearly = 12.0 * 42.52 * 1.025f64.powi(10) * net_share(&p, 2036.0);
        assert!(close(value_per_point(&p, 10.0), 20.0 * yearly, 0.01), "{}", value_per_point(&p, 10.0));
    }

    #[test]
    fn value_falls_with_discount_and_rises_with_longevity() {
        let p = retiring_at_horizon_end();
        let base = value_per_point(&p, 10.0);
        let mut q = p.clone();
        q.pension_discount = 0.05;
        assert!(value_per_point(&q, 10.0) < 0.85 * base);
        let mut q = p.clone();
        q.life_expectancy = 23.0;
        assert!(value_per_point(&q, 10.0) > 1.1 * base);
    }

    #[test]
    fn dying_before_retirement_lowers_the_value() {
        // 12 years between the end of the horizon (age 55) and retirement: a few percent die before.
        let mut p = retiring_at_horizon_end();
        p.age = 45.0;
        let yearly = 12.0 * 42.52 * 1.025f64.powi(22) / 1.025f64.powi(12) * net_share(&p, 2048.0);
        let ratio = value_per_point(&p, 10.0) / (20.0 * yearly);
        assert!(ratio > 0.85 && ratio < 0.99, "{ratio}");
    }

    #[test]
    fn past_retirement_age_the_pension_starts_at_the_end_of_the_horizon() {
        let mut p = retiring_at_horizon_end();
        p.age = 65.0; // 75 at the end of the horizon
        let v = value_per_point(&p, 10.0);
        let at_75 = Mortality::calibrated(67.0, 20.0).life_expectancy(75.0);
        let yearly = 12.0 * 42.52 * 1.025f64.powi(10) * net_share(&p, 2028.0);
        assert!(close(v, at_75 * yearly, 0.01), "{v}");
    }
}
