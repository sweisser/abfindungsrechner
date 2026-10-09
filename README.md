# Severance calculator

Compares *staying* in your job with *taking a severance package* and investing it (German tax rules, tariff 2026).

- `core/` – Rust crate with all computations: tax, social security, ALG I, portfolio, simulation,
  break-evens, sensitivity grid and the Monte Carlo analysis (`monte_carlo/`, `random/`).
- `core.js`, `core.wasm.js` – the crate compiled to WASM: wasm-bindgen bindings plus the module as base64
  (committed, so no Rust toolchain is needed just to use the page, and it works when opened from disk).
- `params.js` – parameter spec/defaults for the UI and the WASM loader.
- `index.html` – the UI (plain HTML/CSS/SVG, no framework); open it directly in a browser.
- `tests/` – Node tests of the built web bundle.

Tasks (needs [just](https://github.com/casey/just)):

| | |
|---|---|
| `just setup` | install the WASM target and the matching `wasm-bindgen` CLI |
| `just test` | Rust unit tests |
| `just build` | rebuild `core.js` / `core.wasm.js` |
| `just check` | lint, Rust tests, build and web tests – run before committing |

The browser calls two wasm-bindgen exports, `analyze` and `monteCarlo`: JSON in, JSON out; errors are thrown.
`monteCarlo` (2000 runs on every input change) returns the final-outcome summaries plus per-year wealth
percentiles and 40 sample paths per scenario, which the wealth chart draws behind the deterministic run.
Not tax advice.
