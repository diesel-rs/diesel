# Diesel Codegen

This crate implements Diesel's procedural macros using the Macros 1.1 system. It
requires nightly Rust from October 10, 2016 or later. For usage on stable
Rust, see
[`diesel_codegen_syntex`](https://github.com/diesel-rs/diesel/tree/master/diesel_codegen_syntex).

Diesel Codegen provides custom derive implementations for
[`Queryable`][queryable], [`Identifiable`][identifiable],
[`Insertable`][insertable], [`AsChangeset`][as-changeset], and [`Associations`].
It also provides the macros [`infer_schema!`][infer-schema],
[`infer_table_from_schema!`][infer-table-from-schema], and
[`embed_migrations!`][embed-migrations].

[queryable]: http://docs.diesel.rs/diesel/query_source/trait.Queryable.html
[identifiable]: http://docs.diesel.rs/diesel/associations/trait.Identifiable.html
[as-changeset]: http://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html
[infer-schema]: http://docs.diesel.rs/diesel/macro.infer_schema!.html
[infer-table-from-schema]: http://docs.diesel.rs/diesel/macro.infer_table_from_schema!.html
[embed-migrations]: http://docs.diesel.rs/diesel/macro.embed_migrations!.html

# Using this crate

First, add this crate to Cargo.toml as so:

```toml
diesel_codegen = { version = "0.9.0", features = ["postgres"] }
```

If you are using SQLite, be sure to specify `sqlite` instead of `postgres` in
the `features` section.

Next, at the root of your crate add:

```rust
#![feature(proc_macro)]

#[macro_use] extern crate diesel_codegen;
```

See the documentation for each trait/macro for additional details and
configuration options.
