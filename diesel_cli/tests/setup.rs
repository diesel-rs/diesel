use support::{database, project};

#[test]
fn setup_creates_database() {
    let p = project("setup_creates_database").build();
    let db = database(&p.database_url());

    // sanity check
    assert!(!db.exists());

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.exists());
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
    let db = database(&p.database_url());
    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("__diesel_schema_migrations"));
}

#[test]
fn setup_runs_migrations_if_no_schema_table() {
    let p = project("setup_runs_migrations_if_no_schema_table")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    // sanity check
    assert!(!db.exists());

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}", result.stdout());
    assert!(db.table_exists("users"));
}

#[test]
fn setup_doesnt_run_migrations_if_schema_table_exists() {
    let p = project("setup_doesnt_run_migrations_if_schema_table_exists")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();
    db.execute("CREATE TABLE __diesel_schema_migrations ( version INTEGER )");

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("users"));
}

#[test]
fn setup_notifies_when_creating_a_database() {
    let p = project("setup_notifies").build();

    let result = p.command("setup").run();

    assert!(result.stdout().contains("Creating database:"),
        "Unexpected stdout {}", result.stdout());
}

#[test]
#[allow(unused_variables)]
fn setup_doesnt_notify_when_not_creating_a_database() {
    let p = project("setup_doesnt_notify").build();
    let db = database(&p.database_url()).create();

    let result = p.command("setup").run();

    assert!(!result.stdout().contains("Creating database:"),
        "Unexpected stdout {}", result.stdout());
}

#[test]
fn setup_works_with_migration_dir_by_arg() {
    let p = project("setup_works_with_migration_dir_by_arg").build();

    // make sure the project builder doesn't create it for us
    assert!(!p.has_file("migrations"));
    assert!(!p.has_file("foo"));

    let result = p.command("setup").arg("--migration-dir=foo").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("migrations"));
    assert!(p.has_file("foo"));
}

#[test]
fn setup_works_with_migration_dir_by_env() {
    let p = project("setup_works_with_migration_dir_by_env").build();

    // make sure the project builder doesn't create it for us
    assert!(!p.has_file("migrations"));
    assert!(!p.has_file("bar"));

    let result = p.command("setup")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("migrations"));
    assert!(p.has_file("bar"));
}
