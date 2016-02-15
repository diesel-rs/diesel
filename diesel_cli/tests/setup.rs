use support::project;
use support::database::*;

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

#[test]
fn setup_creates_migrations_directory() {
    let p = project("setup_creates_migrations_directory").build();

    // make sure the project builder doesn't create it for us
    assert!(!p.has_file("migrations"));

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file("migrations"));
}

#[test]
fn setup_creates_schema_table() {
    let p = project("setup_creates_schema_table").build();

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(table_exists(&p.database_url(), "__diesel_schema_migrations"));
}

#[test]
fn setup_runs_migrations_if_no_schema_table() {
    let p = project("setup_runs_migrations_if_no_schema_table")
        .folder("migrations")
        .build();

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    assert!(!database_exists(&p.database_url()));

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}", result.stdout());
    assert!(table_exists(&p.database_url(), "users"));
}
