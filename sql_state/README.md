# sql-state

[![Crates.io](https://img.shields.io/crates/v/sql-state.svg)](https://crates.io/crates/sql-state)
[![Documentation](https://docs.rs/sql-state/badge.svg)](https://docs.rs/sql-state)

[SQLSTATE](https://en.wikipedia.org/wiki/SQLSTATE) codes with PostgreSQL, MySQL and MariaDB constants.

```rust
let code: sql_state::SqlState = "40001".parse().unwrap();
assert!(matches!(code, sql_state::pg::T_R_SERIALIZATION_FAILURE));
```
