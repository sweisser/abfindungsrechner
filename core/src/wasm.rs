//! Browser bindings (wasm-bindgen). Thin wrappers around `api`; errors become JS exceptions.

use wasm_bindgen::prelude::*;

use crate::api;

/// JSON `Params` in, JSON `Analysis` out.
#[wasm_bindgen]
pub fn analyze(input: &str) -> Result<String, JsError> {
    api::analyze_json(input).map_err(|e| JsError::new(&e))
}

/// JSON `{ params, uncertainty?, runs?, seed? }` in, JSON `MonteCarlo` out.
#[wasm_bindgen(js_name = monteCarlo)]
pub fn monte_carlo(input: &str) -> Result<String, JsError> {
    api::monte_carlo_json(input).map_err(|e| JsError::new(&e))
}

/// JSON `{ params, uncertainty?, runs?, seed? }` in, the Excel workbook (xlsx bytes) out.
#[wasm_bindgen(js_name = exportXlsx)]
pub fn export_xlsx(input: &str) -> Result<Vec<u8>, JsError> {
    api::export_xlsx_json(input).map_err(|e| JsError::new(&e))
}
