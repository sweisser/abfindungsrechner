//! JSON entry points: JSON request in, JSON result out (or an error message).
//!
//! Kept free of any WASM types so they can be tested natively; `wasm` only wraps them.

use serde::Deserialize;

use crate::analysis::analyze_params;
use crate::export;
use crate::monte_carlo::{self, Uncertainty};
use crate::params::Params;

pub const DEFAULT_RUNS: u32 = 2000;
/// Upper bound so a typo cannot freeze the browser.
pub const MAX_RUNS: u32 = 20_000;

/// `Params` in, `Analysis` out.
pub fn analyze_json(input: &str) -> Result<String, String> {
    let p: Params = parse(input)?;
    to_json(&analyze_params(&p))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonteCarloRequest {
    params: Params,
    #[serde(default)]
    uncertainty: Uncertainty,
    /// A number rather than an integer, so that any JSON number is accepted and then clamped.
    runs: Option<f64>,
    #[serde(default)]
    seed: u64,
}

impl MonteCarloRequest {
    fn runs(&self) -> u32 {
        self.runs.map_or(DEFAULT_RUNS, |n| n.clamp(1.0, MAX_RUNS as f64) as u32)
    }
}

/// `MonteCarloRequest` in, `MonteCarlo` out.
pub fn monte_carlo_json(input: &str) -> Result<String, String> {
    let r: MonteCarloRequest = parse(input)?;
    to_json(&monte_carlo::run(&r.params, &r.uncertainty, r.runs(), r.seed))
}

/// `MonteCarloRequest` in, the Excel workbook (xlsx bytes) out: the expected run as formulas, the
/// break-evens, sensitivity grid and this Monte Carlo run as values.
pub fn export_xlsx_json(input: &str) -> Result<Vec<u8>, String> {
    let r: MonteCarloRequest = parse(input)?;
    export::workbook(&r.params, &r.uncertainty, r.runs(), r.seed)
}

fn parse<'a, T: Deserialize<'a>>(input: &'a str) -> Result<T, String> {
    serde_json::from_str(input).map_err(|e| e.to_string())
}

fn to_json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::test_defaults_json;
    use serde_json::{json, Value};

    fn parse(out: String) -> Value {
        serde_json::from_str(&out).unwrap()
    }

    #[test]
    fn analyze_json_rejects_invalid_params() {
        let err = analyze_json("{}").unwrap_err();
        assert!(err.contains("missing field"), "{err}");
    }

    #[test]
    fn analyze_json_returns_analysis() {
        let out = parse(analyze_json(&test_defaults_json().to_string()).unwrap());
        assert!(out["sim"]["realDiff"].is_number());
    }

    fn mc_request(extra: Value) -> Result<String, String> {
        let mut req = json!({ "params": test_defaults_json() });
        req.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        monte_carlo_json(&req.to_string())
    }

    #[test]
    fn monte_carlo_json_uses_defaults_for_optional_fields() {
        let out = parse(mc_request(json!({})).unwrap());
        assert_eq!(out["runs"], DEFAULT_RUNS);
        assert!(out["realDiff"]["probPositive"].is_number());
        assert!(out["realDiff"]["percentiles"]["p50"].is_number());
    }

    #[test]
    fn monte_carlo_json_applies_uncertainty_runs_and_seed() {
        let req = json!({
            "uncertainty": { "newSalary": { "min": 60000, "mode": 80000, "max": 90000 } },
            "runs": 64, "seed": 5
        });
        let out = parse(mc_request(req.clone()).unwrap());
        assert_eq!(out["runs"], 64);
        let p = &out["realDiff"]["percentiles"];
        assert!(p["p95"].as_f64() > p["p5"].as_f64());
        assert_eq!(parse(mc_request(req).unwrap()), out, "same seed, same result");
    }

    #[test]
    fn monte_carlo_json_caps_the_number_of_runs() {
        let out = parse(mc_request(json!({ "runs": 1e9 })).unwrap());
        assert_eq!(out["runs"], MAX_RUNS);
    }

    #[test]
    fn export_xlsx_json_returns_a_workbook() {
        let req = json!({ "params": test_defaults_json(), "runs": 16, "seed": 1 });
        let bytes = export_xlsx_json(&req.to_string()).unwrap();
        assert_eq!(&bytes[..2], b"PK");
    }

    #[test]
    fn export_xlsx_json_rejects_invalid_requests() {
        assert!(export_xlsx_json("{}").unwrap_err().contains("missing field `params`"));
    }

    #[test]
    fn monte_carlo_json_rejects_invalid_requests() {
        assert!(monte_carlo_json("{}").unwrap_err().contains("missing field `params`"));
        let err = mc_request(json!({ "uncertainty": { "newSalary": { "min": 3, "mode": 2, "max": 1 } } })).unwrap_err();
        assert!(err.contains("min ≤ mode ≤ max"), "{err}");
    }
}
