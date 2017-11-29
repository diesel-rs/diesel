# Diesel Derives

This crate implements Diesel's procedural macros using the Macros 1.1 system.
It depends on features introduced in Rust 1.15.
Make sure to always use the latest stable release for optimal performance and feature support.

Diesel Derives provides custom derive implementations for
[`Queryable`][queryable], [`Identifiable`][identifiable],
[`Insertable`][insertable], [`AsChangeset`][as-changeset], and [`Associations`][associations].

[queryable]: http://docs.diesel.rs/diesel/query_source/trait.Queryable.html
[identifiable]: http://docs.diesel.rs/diesel/associations/trait.Identifiable.html
[insertable]: http://docs.diesel.rs/diesel/prelude/trait.Insertable.html
[as-changeset]: http://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html
[associations]: http://docs.diesel.rs/diesel/associations/index.html

# Using this crate

First, add this crate to Cargo.toml as so:

```toml
diesel_derives = { version = "0.16.0", features = ["postgres"] }
```

If you are using SQLite, be sure to specify `sqlite` instead of `postgres` in
the `features` section.

Next, at the root of your crate add:

```rust
#[macro_use] extern crate diesel_derives;
```

See the documentation for each trait/macro for additional details and
configuration options.
