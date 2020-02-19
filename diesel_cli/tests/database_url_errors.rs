#[allow(unused_imports)]
use crate::support::{database, project};

#[test]
#[cfg(not(feature = "sqlite"))]
fn missing_sqlite_panic_bare() {
    let p = project("missing_sqlite_panic_bare").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "example.db")
        .run();
    assert!(result
        .stderr()
        .contains("`example.db` is not a valid database URL. It should start with "));
    assert!(result
        .stderr()
        .contains("or maybe you meant to use the `sqlite` feature which is not enabled."));
}

#[test]
#[cfg(not(feature = "sqlite"))]
fn missing_sqlite_panic_scheme() {
    let p = project("missing_sqlite_panic_scheme").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "sqlite://example.db")
        .run();
    assert!(result
        .stderr()
        .contains("panicked at 'Database url `sqlite://example.db` requires the `sqlite` feature but it's not enabled.'"));
}

#[test]
#[cfg(not(feature = "postgres"))]
fn missing_postgres_panic_postgres() {
    let p = project("missing_postgres_panic_postgres").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "postgres://localhost")
        .run();
    assert!(result
        .stderr()
        .contains("panicked at 'Database url `postgres://localhost` requires the `postgres` feature but it's not enabled.'"));
}

#[test]
#[cfg(not(feature = "postgres"))]
fn missing_postgres_panic_postgresql() {
    let p = project("missing_postgres_panic_postgresql").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "postgresql://localhost")
        .run();
    assert!(result
        .stderr()
        .contains("panicked at 'Database url `postgresql://localhost` requires the `postgres` feature but it's not enabled.'"));
}

#[test]
#[cfg(not(feature = "mysql"))]
fn missing_mysql_panic() {
    let p = project("missing_mysql_panic").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "mysql://localhost")
        .run();
    assert!(result
        .stderr()
        .contains("panicked at 'Database url `mysql://localhost` requires the `mysql` feature but it's not enabled.'"));
}
