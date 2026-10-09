//! Severance-vs-stay simulation (German tax rules, tariff 2026).
//!
//! Pure computation; the browser calls the wasm-bindgen exports in `wasm`
//! (JSON in, JSON out).

pub mod analysis;
pub mod api;
pub mod income;
pub mod monte_carlo;
pub mod params;
pub mod pension;
pub mod portfolio;
pub mod random;
pub mod simulation;
pub mod tax;
pub mod wasm;

pub use analysis::{analyze_params, Analysis};
pub use api::{analyze_json, monte_carlo_json};
pub use params::Params;
pub use simulation::{simulate, Sim};
