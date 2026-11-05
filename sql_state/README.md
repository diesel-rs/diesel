# sql-state

[![Crates.io](https://img.shields.io/crates/v/sql-state.svg)](https://crates.io/crates/sql-state)
[![Documentation](https://docs.rs/sql-state/badge.svg)](https://docs.rs/sql-state)

[SQLSTATE](https://en.wikipedia.org/wiki/SQLSTATE) error codes as a structured type, with named constants for the codes PostgreSQL, MySQL and MariaDB report. The crate is `no_std`, needs no allocator and has no dependencies.

`SqlState` holds any five character code, including custom ones, and its constants are usable as `match` patterns. The `pg` module names the codes PostgreSQL defines, and the `mysql_like` module names the codes MySQL and MariaDB report. Diesel reports database errors with these types and re-exports them as `diesel::result::SqlState`, `diesel::pg::sql_state` and `diesel::mysql_like::sql_state`.

```rust
use sql_state::{SqlState, SqlStateClass, mysql_like, pg};

let code: SqlState = "40001".parse().unwrap();
assert!(matches!(code, pg::T_R_SERIALIZATION_FAILURE));
assert!(matches!(code, mysql_like::SERIALIZATION_FAILURE));
assert_eq!(code.class(), SqlStateClass::TRANSACTION_ROLLBACK);
```
