use support::{database, project};

#[test]
fn database_setup_creates_database() {
    let p = project("database_setup_creates_database")
        .folder("migrations")
        .build();

    let db = database(&p.database_url());

    // sanity check
    assert!(!db.exists());

    let result = p.command("database")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Creating database:"),
        "Unexpected stdout {}", result.stdout());
    assert!(db.exists());
}

#[test]
fn database_setup_creates_schema_table () {
    let p = project("database_setup_creates_schema_table")
        .folder("migrations")
        .build();

    let db = database(&p.database_url());

    // sanity check
    assert!(!db.exists());

    let result = p.command("database")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("__diesel_schema_migrations"));
}

#[test]
fn database_setup_runs_migrations_if_no_schema_table() {
    let p = project("database_setup_runs_migrations_if_no_schema_table")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    // sanity check
    assert!(!db.exists());

    let result = p.command("database")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}", result.stdout());
    assert!(db.table_exists("users"));
}

#[test]
fn database_abbreviated_as_db() {
    let p = project("database_abbreviated_as_db").folder("migrations").build();
    let db = database(&p.database_url());

    let result = p.command("db")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Creating database:"),
        "Unexpected stdout {}", result.stdout());
    assert!(db.exists());
}
