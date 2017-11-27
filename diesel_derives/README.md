# Diesel Derives

This crate implements Diesel's procedural macros using the Macros 1.1 system.
It depends on features introduced in Rust 1.15.
Make sure to always use the latest stable release for optimal performance and feature support.
The functionality of this crate is reexported by diesel, so there is no need to depend directly on this crate.

Diesel Derive provides custom derive implementations for
[`Queryable`][queryable], [`Identifiable`][identifiable],
[`Insertable`][insertable], [`AsChangeset`][as-changeset], and [`Associations`][associations].
[queryable]: https://docs.diesel.rs/diesel/query_source/trait.Queryable.html
[identifiable]: https://docs.diesel.rs/diesel/associations/trait.Identifiable.html
[insertable]: https://docs.diesel.rs/diesel/prelude/trait.Insertable.html
[as-changeset]: https://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html
[associations]: https://docs.diesel.rs/diesel/associations/index.html
