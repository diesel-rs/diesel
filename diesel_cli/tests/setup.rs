use support::project;

#[test]
fn setup_creates_database() {
    let p = project("setup_creates_database").build();

    // sanity check
    assert!(!database_exists(&p.database_url()));

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Creating database:"),
        "Unexpected stdout {}", result.stdout());
    assert!(database_exists(&p.database_url()));
}

#[cfg(feature = "postgres")]
fn database_exists(url: &str) -> bool {
    use diesel::pg::PgConnection;
    use diesel::prelude::*;
    PgConnection::establish(url).is_ok()
}

#[cfg(feature = "sqlite")]
fn database_exists(url: &str) -> bool {
    use std::path::Path;
    Path::new(url).exists()
}
