# sqlstate

[![Crates.io](https://img.shields.io/crates/v/sqlstate.svg)](https://crates.io/crates/sqlstate)
[![Documentation](https://docs.rs/sqlstate/badge.svg)](https://docs.rs/sqlstate)

[SQLSTATE](https://en.wikipedia.org/wiki/SQLSTATE) codes with PostgreSQL, MySQL and MariaDB constants.

```rust
let code: sqlstate::SqlState = "40001".parse().unwrap();
assert!(matches!(code, sqlstate::pg::T_R_SERIALIZATION_FAILURE));
```
