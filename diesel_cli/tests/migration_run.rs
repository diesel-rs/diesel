use diesel::dsl::sql;
use diesel::sql_types::Bool;
use diesel::{select, RunQueryDsl};
use std::path::Path;
use support::{database, project};

#[test]
fn migration_run_runs_pending_migrations() {
    let p = project("migration_run").folder("migrations").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migration_run_inserts_run_on_timestamps() {
    let p = project("migration_run_on_timestamps")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    // Make sure the project is setup.
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    let migrations_done: bool = select(sql::<Bool>(
        "EXISTS (SELECT * FROM __diesel_schema_migrations WHERE version >= '1')",
    ))
    .get_result(&db.conn())
    .unwrap();
    assert!(!migrations_done, "Migrations table should be empty");

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("users"));

    // By running a query that compares timestamps, we are also checking
    // that the auto-inserted values for the "run_on" column are valid.

    #[cfg(feature = "sqlite")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < DATETIME('now', '+1 hour'))",
        ))
        .get_result(&db.conn())
        .unwrap()
    }

    #[cfg(feature = "postgres")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL '1 hour')",
        ))
        .get_result(&db.conn())
        .unwrap()
    }

    #[cfg(feature = "mysql")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL 1 HOUR)",
        ))
        .get_result(&db.conn())
        .unwrap()
    }

    assert!(
        valid_run_on_timestamp(&db),
        "Running a migration did not insert an updated run_on value"
    );
}

#[test]
fn empty_migrations_are_not_valid() {
    let p = project("migration_run_empty").folder("migrations").build();

    p.command("setup").run();

    p.create_migration("12345_empty_migration", "", "");

    let result = p.command("migration").arg("run").run();

    assert!(!result.is_success());
    assert!(result
        .stderr()
        .contains("Failed with: Attempted to run an empty migration."));
}

#[test]
fn error_migrations_fails() {
    let p = project("run_error_migrations_fails")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "run_error_migrations_fails",
        "CREATE TABLE users (id INTEGER PRIMARY KEY}",
        "DROP TABLE users",
    );

    let result = p.command("migration").arg("run").run();

    assert!(!result.is_success());
    assert!(result.stderr().contains("Failed with: "));
}

#[test]
fn output_contains_path_to_migration_script() {
    let p = project("output_contains_path_to_migration_script")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration("output_contains_path_to_migration_script", "", "");

    let result = p.command("migration").arg("run").run();

    assert!(!result.is_success());
    assert!(result.stdout().contains("up.sql"));
}

#[test]
fn any_pending_migrations_works() {
    let p = project("any_pending_migrations_one")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    let result = p.command("migration").arg("pending").run();

    assert!(result.stdout().contains("true\n"));
}

#[test]
fn any_pending_migrations_after_running() {
    let p = project("any_pending_migrations")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    p.command("migration").arg("run").run();

    let result = p.command("migration").arg("pending").run();

    assert!(result.stdout().contains("false\n"));
}

#[test]
fn any_pending_migrations_after_running_and_creating() {
    let p = project("any_pending_migrations_run_then_create")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    p.command("migration").arg("run").run();

    p.create_migration(
        "123456_create_posts_table",
        "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
        "DROP TABLE posts",
    );

    let result = p.command("migration").arg("pending").run();

    assert!(result.stdout().contains("true\n"));
}

#[test]
fn migration_run_runs_pending_migrations_custom_database_url_1() {
    let p = project("migration_run_custom_db_url_1")
        .folder("migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command_without_database_url("migration")
        .arg("run")
        .arg("--database-url")
        .arg(db_url)
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
fn migration_run_runs_pending_migrations_custom_database_url_2() {
    let p = project("migration_run_custom_db_url_2")
        .folder("migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command_without_database_url("migration")
        .arg("--database-url")
        .arg(db_url)
        .arg("run")
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
fn migration_run_runs_pending_migrations_custom_migration_dir_1() {
    let p = project("migration_run_custom_migration_dir_1")
        .folder("custom_migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command("migration")
        .arg("run")
        .arg("--migration-dir")
        .arg(p.migration_dir_in_directory("custom_migrations"))
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
fn migration_run_runs_pending_migrations_custom_migration_dir_2() {
    let p = project("migration_run_custom_migration_dir_2")
        .folder("custom_migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command("migration")
        .arg("--migration-dir")
        .arg(p.migration_dir_in_directory("custom_migrations"))
        .arg("run")
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
fn migration_run_updates_schema_if_config_present() {
    let p = project("migration_run_updates_schema_if_config_present")
        .folder("migrations")
        .file(
            "diesel.toml",
            r#"
            [print_schema]
            file = "src/my_schema.rs"
            "#,
        )
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!p.has_file("src/my_schema.rs"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(p.has_file("src/my_schema.rs"));
}

#[test]
fn migrations_can_be_run_with_no_config_file() {
    let p = project("migration_run_no_config_file")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.command("database").arg("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migrations_can_be_run_with_no_cargo_toml() {
    let p = project("migration_run_no_cargo_toml")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    let cargo_toml_path = Path::new("Cargo.toml");
    p.delete_single_file(&cargo_toml_path);

    p.command("database").arg("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn verify_schema_errors_if_schema_file_would_change() {
    let p = project("migration_run_verify_schema_errors")
        .folder("migrations")
        .file(
            "diesel.toml",
            r#"
            [print_schema]
            file = "src/my_schema.rs"
            "#,
        )
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        "DROP TABLE users",
    );

    assert!(!p.has_file("src/my_schema.rs"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file("src/my_schema.rs"));

    p.create_migration(
        "12346_create_posts_table",
        "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
        "DROP TABLE posts",
    );

    let result = p
        .command("migration")
        .arg("run")
        .arg("--locked-schema")
        .run();

    assert!(
        !result.is_success(),
        "Result was successful, expected to fail {:?}",
        result
    );
    assert!(
        result
            .stderr()
            .contains("Command would result in changes to src/my_schema.rs"),
        "Unexpected stderr {}",
        result.stderr()
    );
    assert!(p.has_file("src/my_schema.rs"));
}
