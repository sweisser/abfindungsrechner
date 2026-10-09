//! Deterministic analysis around one simulation: break-evens and a sensitivity grid.

use serde::Serialize;

use crate::params::Params;
use crate::simulation::{simulate, Sim};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Breakeven {
    /// `None`: B never catches up within the search range.
    pub value: Option<f64>,
    /// B already wins at the lower bound of the search range.
    pub at_lower_bound: bool,
}

/// Bisection on one parameter (diff is monotone increasing in it) until B == A.
fn breakeven(p: &Params, set: impl Fn(&mut Params, f64), lo: f64, hi: f64) -> Breakeven {
    let f = |v: f64| {
        let mut q = p.clone();
        set(&mut q, v);
        simulate(&q).diff
    };
    let (mut lo, mut hi) = (lo, hi);
    if f(lo) >= 0.0 {
        return Breakeven { value: Some(lo), at_lower_bound: true };
    }
    if f(hi) < 0.0 {
        return Breakeven { value: None, at_lower_bound: false };
    }
    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Breakeven { value: Some((lo + hi) / 2.0), at_lower_bound: false }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Grid {
    pub salaries: Vec<f64>,
    pub months: Vec<f64>,
    /// `cells[row][col]`: real final wealth of B minus A.
    pub cells: Vec<Vec<f64>>,
}

fn sensitivity(p: &Params) -> Grid {
    let salaries: Vec<f64> =
        [0.6, 0.8, 1.0, 1.2].iter().map(|m| (p.new_salary * m / 1000.0).round() * 1000.0).collect();
    let months = vec![0.0, 3.0, 6.0, 12.0, 18.0, 24.0];
    let cells = salaries
        .iter()
        .map(|&s| {
            months
                .iter()
                .map(|&m| {
                    let mut q = p.clone();
                    q.new_salary = s;
                    q.search_months = m;
                    simulate(&q).real_diff
                })
                .collect()
        })
        .collect();
    Grid { salaries, months, cells }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub sim: Sim,
    pub be_new_salary: Breakeven,
    pub be_severance: Breakeven,
    pub grid: Grid,
}

pub fn analyze_params(p: &Params) -> Analysis {
    Analysis {
        sim: simulate(p),
        be_new_salary: breakeven(p, |q, v| q.new_salary = v, 0.0, 500_000.0),
        be_severance: breakeven(p, |q, v| q.severance = v, 0.0, 3e6),
        grid: sensitivity(p),
    }
}
