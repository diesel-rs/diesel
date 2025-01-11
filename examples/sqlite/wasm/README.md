# `rs-diesel-sqlite`

Diesel's `Getting Started` guide using SQLite instead of Postgresql

## Usage

```
rustup target add wasm32-unknown-unknown
# Add wasm32-unknown-unknown toolchain

cargo install wasm-pack

wasm-pack build --web
# Build wasm

python3 -m http.server 8000
# Next, use it on the web page
```
