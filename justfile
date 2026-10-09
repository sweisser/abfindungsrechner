# Severance calculator – common tasks. Run `just` to list them.

core := "core/Cargo.toml"
wasm := "core/target/wasm32-unknown-unknown/release/abfindung_core.wasm"
bindgen_out := "core/target/web"
# The wasm-bindgen CLI must match the crate version pinned in core/Cargo.toml.
bindgen_version := `grep -A1 '^name = "wasm-bindgen"$' core/Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p'`

default:
    @just --list

# Install the WASM target and the matching wasm-bindgen CLI
setup:
    rustup target add wasm32-unknown-unknown
    cargo install wasm-bindgen-cli --version {{bindgen_version}} --locked

# Run the Rust unit tests
test:
    cargo test --manifest-path {{core}}

# Format check and clippy (warnings are errors)
lint:
    cargo fmt --manifest-path {{core}} --check
    cargo clippy --manifest-path {{core}} --all-targets -- -D warnings

# Format the Rust code
fmt:
    cargo fmt --manifest-path {{core}}

# Compile the core to WASM: core.js (bindings) + core.wasm.js (module as base64)
build:
    cargo build --manifest-path {{core}} --release --target wasm32-unknown-unknown
    wasm-bindgen --target no-modules --no-typescript --out-dir {{bindgen_out}} --out-name core {{wasm}}
    cp {{bindgen_out}}/core.js core.js
    { printf 'window.CORE_WASM_B64="'; base64 -w0 {{bindgen_out}}/core_bg.wasm; printf '";\n'; } > core.wasm.js
    @echo "wrote core.js and core.wasm.js ($(stat -c %s {{bindgen_out}}/core_bg.wasm) bytes wasm)"

# Test the built web bundle in Node
test-web: build
    node --test tests/*.test.js

# Everything before a commit: lint, Rust tests, build, web tests
check: lint test test-web

# Open the calculator in the browser
open:
    xdg-open index.html
