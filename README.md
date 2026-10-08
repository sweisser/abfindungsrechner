# Severance calculator

Compares *staying* in your job with *taking a severance package* and investing it (German tax rules, tariff 2026).

- `core/` – Rust crate with all computations (tax, social security, ALG I, portfolio, break-evens, sensitivity grid). `cargo test` runs its tests.
- `build.sh` – compiles the crate to WASM and embeds it as base64 in `core.wasm.js` (committed, so no Rust toolchain is needed just to use the page).
- `params.js` – parameter spec/defaults for the UI and the small WASM loader.
- `index.html` – Bootstrap UI; open it directly in a browser.

After changing `core/src/lib.rs`: `cargo test --manifest-path core/Cargo.toml && ./build.sh`.

The WASM interface is a minimal C ABI (`alloc`, `analyze`, `out_len`, `dealloc`): JSON parameters in, JSON analysis out.
Not tax advice.
