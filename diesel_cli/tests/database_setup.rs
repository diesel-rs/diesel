use crate::support::{database, project};

#[test]
fn database_setup_creates_database() {
    let p = project("database_setup_creates_database")
        .folder("migrations")
        .build();

    let db = database(&p.database_url());

    // sanity check
    assert!(!db.exists());

    let result = p.command("database").arg("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Creating database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.exists());
}

#[test]
fn database_setup_creates_schema_table() {
    let p = project("database_setup_creates_schema_table")
        .folder("migrations")
        .build();

    let db = database(&p.database_url());

    // sanity check
    assert!(!db.exists());

    let result = p.command("database").arg("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("__diesel_schema_migrations"));
}

#[test]
fn database_setup_runs_migrations_if_no_schema_table() {
    let p = project("database_setup_runs_migrations_if_no_schema_table")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p.command("database").arg("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn database_abbreviated_as_db() {
    let p = project("database_abbreviated_as_db")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    let result = p.command("db").arg("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Creating database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.exists());
}

#[test]
fn database_setup_respects_migration_dir_by_arg_to_database() {
    let p = project("database_setup_respects_migration_dir_by_arg_to_database")
        .folder("foo")
        .build();

    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p
        .command("database")
        .arg("--migration-dir=foo")
        .arg("setup")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn database_setup_respects_migration_dir_by_arg() {
    let p = project("database_setup_respects_migration_dir_by_arg")
        .folder("foo")
        .build();
    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p
        .command("database")
        .arg("setup")
        .arg("--migration-dir=foo")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn database_setup_respects_migration_nested_dir_by_arg() {
    let p = project("database_setup_respects_migration_nested_dir_by_arg")
        .folder("foo/bar")
        .build();
    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "foo/bar",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p
        .command("database")
        .arg("setup")
        .arg("--migration-dir=foo/bar")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn database_setup_respects_migration_dir_by_env() {
    let p = project("database_setup_respects_migration_dir_by_env")
        .folder("bar")
        .build();
    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "bar",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p
        .command("database")
        .arg("setup")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn database_setup_respects_migrations_dir_from_diesel_toml() {
    let p = project("database_setup_respects_migrations_dir_by_diesel_toml")
        .folder("custom_migrations")
        .file(
            "diesel.toml",
            r#"
            [migrations_directory]
            dir = "custom_migrations"
            "#,
        )
        .build();
    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p.command("database").arg("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}
