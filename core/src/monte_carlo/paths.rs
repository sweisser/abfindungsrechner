//! Wealth year by year across many runs: percentile bands plus a few sample paths.

use serde::Serialize;

use super::stats::{percentiles, Percentiles};
use crate::income::YearRow;

/// One quantity over time; index 0 = start, index k = end of year k.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PathStats {
    /// Percentiles across all runs, per year.
    pub percentiles: Vec<Percentiles>,
    /// The first few runs, unchanged, to show what single futures look like.
    pub samples: Vec<Vec<f64>>,
}

/// Wealth of one scenario, before liquidation tax.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WealthPaths {
    pub nominal: PathStats,
    /// In today's money, deflated along each run's own inflation path.
    pub real: PathStats,
}

/// Accumulates equally long paths and keeps the first `max_samples` of them.
pub struct PathCollector {
    by_year: Vec<Vec<f64>>,
    samples: Vec<Vec<f64>>,
    max_samples: usize,
}

impl PathCollector {
    pub fn new(max_samples: usize) -> Self {
        PathCollector { by_year: Vec::new(), samples: Vec::new(), max_samples }
    }

    pub fn push(&mut self, path: Vec<f64>) {
        if self.by_year.is_empty() {
            self.by_year = vec![Vec::new(); path.len()];
        }
        assert_eq!(path.len(), self.by_year.len(), "paths must have the same length");
        for (year, &v) in self.by_year.iter_mut().zip(&path) {
            year.push(v);
        }
        if self.samples.len() < self.max_samples {
            self.samples.push(path);
        }
    }

    pub fn finish(self) -> PathStats {
        PathStats { percentiles: self.by_year.iter().map(|xs| percentiles(xs)).collect(), samples: self.samples }
    }
}

/// Collects nominal and real wealth of one scenario.
pub struct WealthCollector {
    nominal: PathCollector,
    real: PathCollector,
}

impl WealthCollector {
    pub fn new(max_samples: usize) -> Self {
        WealthCollector { nominal: PathCollector::new(max_samples), real: PathCollector::new(max_samples) }
    }

    /// `price_levels` as in `Sim`: one more entry than `rows`, starting at 1.
    pub fn push(&mut self, start: f64, rows: &[YearRow], price_levels: &[f64]) {
        let nominal = wealth_path(start, rows);
        self.real.push(deflate(&nominal, price_levels));
        self.nominal.push(nominal);
    }

    pub fn finish(self) -> WealthPaths {
        WealthPaths { nominal: self.nominal.finish(), real: self.real.finish() }
    }
}

/// Starting wealth followed by the wealth at the end of each year.
fn wealth_path(start: f64, rows: &[YearRow]) -> Vec<f64> {
    std::iter::once(start).chain(rows.iter().map(|r| r.wealth)).collect()
}

fn deflate(path: &[f64], price_levels: &[f64]) -> Vec<f64> {
    assert_eq!(path.len(), price_levels.len(), "one price level per point of the path");
    path.iter().zip(price_levels).map(|(v, l)| v / l).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(v: f64) -> Percentiles {
        Percentiles { p5: v, p25: v, p50: v, p75: v, p95: v }
    }

    #[test]
    fn percentiles_are_taken_per_year() {
        let mut c = PathCollector::new(0);
        for i in 0..=100 {
            c.push(vec![7.0, i as f64, 2.0 * i as f64]);
        }
        let s = c.finish();
        assert_eq!(s.percentiles.len(), 3);
        assert_eq!(s.percentiles[0], flat(7.0));
        assert_eq!(s.percentiles[1], Percentiles { p5: 5.0, p25: 25.0, p50: 50.0, p75: 75.0, p95: 95.0 });
        assert_eq!(s.percentiles[2].p50, 100.0);
        assert!(s.samples.is_empty());
    }

    #[test]
    fn keeps_the_first_paths_as_samples() {
        let mut c = PathCollector::new(2);
        for i in 0..5 {
            c.push(vec![i as f64; 2]);
        }
        assert_eq!(c.finish().samples, vec![vec![0.0, 0.0], vec![1.0, 1.0]]);
    }

    #[test]
    #[should_panic(expected = "same length")]
    fn rejects_paths_of_different_length() {
        let mut c = PathCollector::new(0);
        c.push(vec![1.0, 2.0]);
        c.push(vec![1.0]);
    }

    fn row(wealth: f64) -> YearRow {
        YearRow { wealth, ..Default::default() }
    }

    #[test]
    fn wealth_starts_with_the_initial_value_and_is_deflated_per_year() {
        let mut c = WealthCollector::new(1);
        c.push(100.0, &[row(110.0), row(132.0)], &[1.0, 1.1, 1.2]);
        let w = c.finish();
        assert_eq!(w.nominal.samples, vec![vec![100.0, 110.0, 132.0]]);
        let real = &w.real.samples[0];
        assert_eq!(real[0], 100.0);
        assert!((real[1] - 100.0).abs() < 1e-9);
        assert!((real[2] - 110.0).abs() < 1e-9);
    }
}
