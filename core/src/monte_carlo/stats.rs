//! Summary statistics of a sample of outcomes.

use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
pub struct Percentiles {
    pub p5: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
}

/// Equal-width bins; bin `i` spans `edges[i]..edges[i + 1]` (the last one includes its upper edge).
#[derive(Serialize, Debug, PartialEq)]
pub struct Histogram {
    pub edges: Vec<f64>,
    pub counts: Vec<u32>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub mean: f64,
    pub percentiles: Percentiles,
    /// Share of outcomes above zero.
    pub prob_positive: f64,
    pub histogram: Histogram,
}

/// Summarize a non-empty sample.
pub fn summarize(values: &[f64], bins: usize) -> Summary {
    assert!(!values.is_empty(), "cannot summarize an empty sample");
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = values.len() as f64;
    Summary {
        mean: values.iter().sum::<f64>() / n,
        percentiles: percentiles_sorted(&sorted),
        prob_positive: values.iter().filter(|&&x| x > 0.0).count() as f64 / n,
        histogram: histogram(&sorted, bins),
    }
}

/// Percentiles of a non-empty sample.
pub fn percentiles(values: &[f64]) -> Percentiles {
    assert!(!values.is_empty(), "cannot take percentiles of an empty sample");
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    percentiles_sorted(&sorted)
}

fn percentiles_sorted(sorted: &[f64]) -> Percentiles {
    let at = |q| percentile(sorted, q);
    Percentiles { p5: at(0.05), p25: at(0.25), p50: at(0.5), p75: at(0.75), p95: at(0.95) }
}

/// Linear interpolation between the closest ranks of a sorted sample.
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let pos = q * (sorted.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

fn histogram(sorted: &[f64], bins: usize) -> Histogram {
    let (min, max) = (sorted[0], sorted[sorted.len() - 1]);
    if min == max || bins == 0 {
        return Histogram { edges: vec![min, max], counts: vec![sorted.len() as u32] };
    }
    let width = (max - min) / bins as f64;
    let edges = (0..=bins).map(|i| min + width * i as f64).collect();
    let mut counts = vec![0; bins];
    for &x in sorted {
        let bin = (((x - min) / width) as usize).min(bins - 1);
        counts[bin] += 1;
    }
    Histogram { edges, counts }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_interpolate_between_sorted_values() {
        let mut xs: Vec<f64> = (0..=100).map(f64::from).collect();
        xs.reverse();
        let s = summarize(&xs, 10);
        assert_eq!(s.percentiles, Percentiles { p5: 5.0, p25: 25.0, p50: 50.0, p75: 75.0, p95: 95.0 });
        assert_eq!(summarize(&[1.0, 2.0], 1).percentiles.p50, 1.5);
    }

    #[test]
    fn percentiles_of_an_unsorted_sample() {
        let p = percentiles(&[4.0, 0.0, 2.0, 1.0, 3.0]);
        assert_eq!(p, Percentiles { p5: 0.2, p25: 1.0, p50: 2.0, p75: 3.0, p95: 3.8 });
    }

    #[test]
    fn mean_and_share_of_positive_outcomes() {
        let s = summarize(&[-1.0, 0.0, 1.0, 4.0], 2);
        assert_eq!(s.mean, 1.0);
        assert_eq!(s.prob_positive, 0.5);
    }

    #[test]
    fn histogram_counts_every_value_once() {
        let xs = [0.0, 1.0, 2.5, 5.0, 9.9, 10.0];
        let h = summarize(&xs, 4).histogram;
        assert_eq!(h.edges, vec![0.0, 2.5, 5.0, 7.5, 10.0]);
        assert_eq!(h.counts, vec![2, 1, 1, 2]);
    }

    #[test]
    fn constant_sample_has_a_single_bin() {
        let h = summarize(&[3.0; 5], 20).histogram;
        assert_eq!(h, Histogram { edges: vec![3.0, 3.0], counts: vec![5] });
    }
}
