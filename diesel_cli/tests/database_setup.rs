use support::project;
use support::database::*;

#[test]
fn database_setup_creates_database() {
    let p = project("database_setup_creates_database")
        .folder("migrations")
        .build();

    // sanity check
    assert!(!database_exists(&p.database_url()));

    let result = p.command("database")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Creating database:"),
        "Unexpected stdout {}", result.stdout());
    assert!(database_exists(&p.database_url()));
}

#[test]
fn database_setup_creates_schema_table () {
    let p = project("database_setup_creates_schema_table")
        .folder("migrations")
        .build();

    // sanity check
    assert!(!database_exists(&p.database_url()));

    let result = p.command("database")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(table_exists(&p.database_url(), "__diesel_schema_migrations"));
}

#[test]
fn database_setup_runs_migrations_if_no_schema_table() {
    let p = project("database_setup_runs_migrations_if_no_schema_table")
        .folder("migrations")
        .build();

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    assert!(!database_exists(&p.database_url()));

    let result = p.command("database")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}", result.stdout());
    assert!(table_exists(&p.database_url(), "users"));
}
