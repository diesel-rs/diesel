# `rs-diesel-sqlite`

Diesel's `Getting Started` guide using SQLite instead of Postgresql

## Usage

Compile wasm and start the web server:

```
rustup target add wasm32-unknown-unknown
# Add wasm32-unknown-unknown toolchain

cargo install wasm-pack
# Install the wasm-pack toolchain

wasm-pack build --target web
# Build wasm

python3 server.py
# Start server
```

Next, try it on the web page: [on the web page](http://localhost:8000)
