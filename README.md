[![](https://diesel.rs/assets/images/diesel_logo_stacked_black.png)](https://diesel.rs)

A safe, extensible ORM and Query Builder for Rust
==========================================================
[![Build Status](https://github.com/diesel-rs/diesel/workflows/CI%20Tests/badge.svg)](https://github.com/diesel-rs/diesel/actions?query=workflow%3A%22CI+Tests%22+branch%3Amaster)
[![Gitter](https://badges.gitter.im/diesel-rs/diesel.svg)](https://gitter.im/diesel-rs/diesel?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![Crates.io](https://img.shields.io/crates/v/diesel.svg)](https://crates.io/crates/diesel)

API Documentation: [latest release](https://docs.rs/diesel) – [master branch](https://docs.diesel.rs/master/diesel/index.html)

[Homepage](https://diesel.rs)

Diesel gets rid of the boilerplate for database interaction and eliminates
runtime errors without sacrificing performance. It takes full advantage of
Rust's type system to create a low overhead query builder that "feels like
Rust."

Supported databases:
1. [PostgreSQL](https://docs.diesel.rs/diesel/pg/index.html)
2. [MySQL](https://docs.diesel.rs/diesel/mysql/index.html)
3. [SQLite](https://docs.diesel.rs/diesel/sqlite/index.html)

You can configure the database backend in `Cargo.toml`:
```toml
[dependencies]
diesel = { version = "<version>", features = ["<postgres|mysql|sqlite>"] }
```

## Getting Started

Find our extensive Getting Started tutorial at
[https://diesel.rs/guides/getting-started](https://diesel.rs/guides/getting-started).
Guides on more specific features are coming soon.

## Getting help
If you run into problems, Diesel has a very active Gitter room.
You can come ask for help at
[gitter.im/diesel-rs/diesel](https://gitter.im/diesel-rs/diesel).
For help with longer questions and discussion about the future of Diesel,
open a discussion on [GitHub Discussions](https://github.com/diesel-rs/diesel/discussions).

## Code of conduct

Anyone who interacts with Diesel in any space, including but not limited to
this GitHub repository, must follow our [code of conduct](https://github.com/diesel-rs/diesel/blob/master/code_of_conduct.md).

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

### Contributing
Before contributing, please read the [contributors guide](https://github.com/diesel-rs/diesel/blob/master/CONTRIBUTING.md)
for useful information about setting up Diesel locally, coding style and common abbreviations.

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
