#[cfg(feature = "postgres")]
use std::path::Path;

use crate::support::{database, project};

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
#[cfg(feature = "postgres")]
fn setup_initial_migration_returns_nothing_to_console() {
    let p = project("setup_initial_migration_returns_nothing_to_console").build();

    let result = p.command("setup").run();

    assert!(!result.stdout().contains("Running migration"));
}

#[test]
#[cfg(feature = "postgres")]
fn setup_creates_default_migration_file() {
    let p = project("setup_creates_default_migration_file").build();

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file(Path::new("migrations").join("00000000000000_diesel_initial_setup")));
}

#[test]
#[cfg(feature = "postgres")]
fn setup_creates_default_migration_file_if_project_is_otherwise_setup() {
    let p = project("setup_creates_default_migration_file_if_project_is_otherwise_setup").build();

    let initial_migration_path =
        Path::new("migrations").join("00000000000000_diesel_initial_setup");
    let result = p.command("setup").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    p.delete_file(&initial_migration_path);
    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file(&initial_migration_path));
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

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // sanity check
    assert!(!db.exists());

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn setup_doesnt_run_migrations_if_schema_table_exists() {
    let p = project("setup_doesnt_run_migrations_if_schema_table_exists")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();
    db.execute("CREATE TABLE __diesel_schema_migrations ( version INTEGER )");

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("users"));
}

#[test]
fn setup_notifies_when_creating_a_database() {
    let p = project("setup_notifies").build();

    let result = p.command("setup").run();

    assert!(
        result.stdout().contains("Creating database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
#[allow(unused_variables)]
fn setup_doesnt_notify_when_not_creating_a_database() {
    let p = project("setup_doesnt_notify").build();
    let db = database(&p.database_url()).create();

    let result = p.command("setup").run();

    assert!(
        !result.stdout().contains("Creating database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
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
fn setup_writes_migration_dir_by_arg_to_config_file() {
    let p = project("setup_writes_migration_dir_by_arg_to_config_file").build();

    // make sure the project builder doesn't create it for us
    assert!(!p.has_file("migrations"));
    assert!(!p.has_file("foo"));

    let result = p.command("setup").arg("--migration-dir=foo").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("migrations"));
    assert!(p.has_file("foo"));
    assert!(p.file_contents("diesel.toml").contains("dir = \"foo\""));
}

#[test]
#[cfg(windows)]
fn setup_writes_migration_dir_by_arg_to_config_file_win() {
    let p = project("setup_writes_migration_dir_by_arg_to_config_file_win").build();

    // make sure the project builder doesn't create it for us
    assert!(!p.has_file("migrations"));
    assert!(!p.has_file("foo"));

    let result = p.command("setup").arg("--migration-dir=foo\\bar").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("migrations"));
    assert!(p.has_file("foo"));
    assert!(p
        .file_contents("diesel.toml")
        .contains("dir = \"foo\\\\bar\""));
}

#[test]
fn setup_works_with_migration_dir_by_env() {
    let p = project("setup_works_with_migration_dir_by_env").build();

    // make sure the project builder doesn't create it for us
    assert!(!p.has_file("migrations"));
    assert!(!p.has_file("bar"));

    let result = p.command("setup").env("MIGRATION_DIRECTORY", "bar").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("migrations"));
    assert!(p.has_file("bar"));
}

#[test]
fn setup_creates_config_file() {
    let p = project("setup_creates_config_file").build();

    // Make sure the project builder didn't create the file
    assert!(!p.has_file("diesel.toml"));

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file("diesel.toml"));
    assert!(p
        .file_contents("diesel.toml")
        .contains("diesel.rs/guides/configuring-diesel-cli"));
}

#[test]
fn setup_can_take_config_file_by_env() {
    let p = project("setup_can_take_config_file_by_env").build();

    // Make sure the project builder didn't create the file
    assert!(!p.has_file("diesel.toml"));
    assert!(!p.has_file("foo"));

    let result = p.command("setup").env("DIESEL_CONFIG_FILE", "foo").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("diesel.toml"));
    assert!(p.has_file("foo"));
    assert!(p
        .file_contents("foo")
        .contains("diesel.rs/guides/configuring-diesel-cli"));
}

#[test]
fn setup_can_take_config_file_by_param() {
    let p = project("setup_can_take_config_file_by_param").build();

    // Make sure the project builder didn't create the file
    assert!(!p.has_file("diesel.toml"));
    assert!(!p.has_file("foo"));
    assert!(!p.has_file("bar"));

    let result = p
        .command("setup")
        .env("DIESEL_CONFIG_FILE", "foo")
        .arg("--config-file=bar")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!p.has_file("diesel.toml"));
    assert!(!p.has_file("foo"));
    assert!(p.has_file("bar"));
    assert!(p
        .file_contents("bar")
        .contains("diesel.rs/guides/configuring-diesel-cli"));
}

#[test]
fn setup_respects_migrations_dir_from_diesel_toml() {
    let p = project("setup_respects_migration_dir_from_diesel_toml")
        .file(
            "diesel.toml",
            r#"
            [migrations_directory]
            dir = "custom_migrations"
            "#,
        )
        .build();

    assert!(!p.has_file("custom_migrations"));

    let result = p.command("setup").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file("custom_migrations"));
}
