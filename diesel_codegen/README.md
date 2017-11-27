# Diesel Codegen

This crate implements Diesel's procedural macros using the Macros 1.1 system.
It depends on features introduced in Rust 1.15.
Make sure to always use the latest stable release for optimal performance and feature support.

Diesel Codegen provides custom derive implementations for
[`Queryable`][queryable], [`Identifiable`][identifiable],
[`Insertable`][insertable], [`AsChangeset`][as-changeset], and [`Associations`][associations].
It also provides the macros [`infer_schema!`][infer-schema],
[`infer_table_from_schema!`][infer-table-from-schema], and
[`embed_migrations!`][embed-migrations].

[queryable]: https://docs.diesel.rs/diesel/query_source/trait.Queryable.html
[identifiable]: https://docs.diesel.rs/diesel/associations/trait.Identifiable.html
[insertable]: https://docs.diesel.rs/diesel/prelude/trait.Insertable.html
[as-changeset]: https://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html
[associations]: https://docs.diesel.rs/diesel/associations/index.html
[infer-schema]: https://docs.diesel.rs/diesel/macro.infer_schema!.html
[infer-table-from-schema]: https://docs.diesel.rs/diesel/macro.infer_table_from_schema!.html
[embed-migrations]: https://docs.diesel.rs/diesel/macro.embed_migrations!.html

# Using this crate

First, add this crate to Cargo.toml as so:

```toml
diesel_codegen = { version = "0.16.0", features = ["postgres"] }
```

If you are using SQLite, be sure to specify `sqlite` instead of `postgres` in
the `features` section.

Next, at the root of your crate add:

```rust
#[macro_use] extern crate diesel_codegen;
```

See the documentation for each trait/macro for additional details and
configuration options.
