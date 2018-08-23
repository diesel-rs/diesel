[![](https://diesel.rs/assets/images/diesel_logo_stacked_black.png)](https://diesel.rs)

Query schemas not known at compile time with Diesel
===================================================

[![Build Status](https://travis-ci.org/diesel-rs/diesel.svg)](https://travis-ci.org/diesel-rs/diesel-dynamic-schema)
[![Gitter](https://badges.gitter.im/diesel-rs/diesel.svg)](https://gitter.im/diesel-rs/diesel?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

API Documentation: [latest release](https://docs.rs/diesel-dynamic-schema)

Diesel is built to provide strong compile time guarantees that your queries are
valid. To do this, it needs to represent your schema at compile time. However,
there are some times where you don't actually know the schema you're interacting
with until runtime.

This crate provides tools to work with those cases, while still being able to
use Diesel's query builder. Keep in mind that many compile time guarantees are
lost. We cannot verify that the tables/columns you ask for actually exist, or
that the types you state are correct.

Getting Started
---------------

The main function used by this crate is `table`. Note that you must always
provide an explicit select clause when using this crate.

```rust
use diesel_dynamic_schema::table;

let users = table("users");
let id = users.column::<Integer, _>("id");
let name = users.column::<Text, _>("name");

users.select((id, name))
    .filter(name.eq("Sean"))
    .first(&conn)
```

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

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
