# `rs-diesel-sqlite`

Diesel's `Getting Started` guide using SQLite instead of Postgresql

## Usage

Compile wasm and start the web server:

```
rustup target add wasm32-unknown-unknown
# Add wasm32-unknown-unknown toolchain

cargo install wasm-bindgen-cli --locked
# Install the wasm-bindgen cli

cargo build --target wasm32-unknown-unknown
# Build wasm

wasm-bindgen ../../../target/wasm32-unknown-unknown/debug/sqlite_wasm_example.wasm --out-dir pkg --web
# bindgen

python3 server.py
# Start server
```

Next, try it on the web page: [on the web page](http://localhost:8000)
