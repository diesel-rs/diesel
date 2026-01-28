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
    assert!(result.stderr().contains(
        "Database url `sqlite://example.db` requires the `sqlite` feature but it's not enabled."
    ));
}

#[test]
#[cfg(not(feature = "postgres"))]
fn missing_postgres_panic_postgres() {
    let p = project("missing_postgres_panic_postgres").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "postgres://localhost")
        .run();
    assert!(result.stderr().contains(
        "Database url `postgres://localhost` requires the `postgres` feature but it's not enabled."
    ));
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
        .contains("Database url `postgresql://localhost` requires the `postgres` feature but it's not enabled."));
}

#[test]
#[cfg(not(feature = "mysql"))]
fn missing_mysql_panic() {
    let p = project("missing_mysql_panic").build();
    let result = p
        .command_without_database_url("setup")
        .env("DATABASE_URL", "mysql://localhost")
        .run();
    assert!(result.stderr().contains(
        "Database url `mysql://localhost` requires the `mysql` feature but it's not enabled."
    ));
}

#[test]
fn broken_dotenv_file_results_in_error() {
    #[cfg(feature = "postgres")]
    let url = "postgres://localhost";
    #[cfg(feature = "mysql")]
    let url = "mysql://localhost";
    #[cfg(feature = "sqlite")]
    let url = ":memory:";

    let mut p = project("broken_dotenv_file_results_in_error")
        .file(".env", &format!("DATABASE_URL={url}\n;foo\n#bar"))
        .build();

    p.skip_drop_db();

    let result = p.command_without_database_url("setup").run();
    assert!(result
        .stderr()
        .contains("Initializing `.env` file failed: Error parsing line"));
    assert!(!result.is_success());
}
