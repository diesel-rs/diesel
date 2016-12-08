# Diesel Codegen Syntex

Provides the functionality of `diesel_codegen` using Syntex for usage on stable.

## Getting started

Add `diesel_codegen_syntex` to your `Cargo.toml`, specifying which backends you
use.

```toml
diesel_codegen_syntex = { version = "0.8.0", features = ["postgres"] }
```

Next, move the `mod` declarations of any modules that need codegen to a separate
file, such as `lib.in.rs`, like so:

```rust
// main.in.rs
mod schema;
mod models;
```

```rust
// main.rs
include!(concat!(env!("OUT_DIR"), "/main.rs"));
```

Finally, add a build file which calls `diesel_codgen_syntex::expand`

```rust
// build.rs
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let src = Path::new("src/main.in.rs");
    let dst = Path::new(&out_dir).join("main.rs");
    diesel_codegen_syntex::expand(&src, &dst).unwrap();
}
```

For more examples, please see section 4 of the [getting started
guide](http://diesel.rs/guides/getting-started/)
