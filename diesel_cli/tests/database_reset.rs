#[cfg(feature = "postgres")]
extern crate url;

use crate::support::{database, project};

#[test]
fn reset_drops_the_database() {
    let p = project("reset_drops_the_database")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();
    db.execute("CREATE TABLE posts ( id INTEGER )");

    assert!(db.table_exists("posts"));

    let result = p.command("database").arg("reset").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("posts"));
}

#[test]
fn reset_runs_database_setup() {
    let p = project("reset_runs_database_setup")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();

    db.execute("CREATE TABLE posts ( id INTEGER )");
    db.execute("CREATE TABLE users ( id INTEGER )");
    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    assert!(db.table_exists("posts"));
    assert!(db.table_exists("users"));

    let result = p.command("database").arg("reset").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("posts"));
    assert!(db.table_exists("users"));
    assert!(db.table_exists("__diesel_schema_migrations"));
}

#[test]
#[cfg(feature = "postgres")]
fn reset_handles_postgres_urls_with_username_and_password() {
    let p = project("handles_postgres_urls")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();

    db.execute("DROP ROLE IF EXISTS foo");
    db.execute("CREATE ROLE foo WITH LOGIN SUPERUSER PASSWORD 'password'");

    let database_url = {
        let mut new_url = url::Url::parse(&p.database_url()).expect("invalid url");
        new_url.set_username("foo").expect("could not set username");
        new_url
            .set_password(Some("password"))
            .expect("could not set password");
        new_url.to_string()
    };

    let result = p
        .command("database")
        .arg("reset")
        .env("DATABASE_URL", &database_url)
        .run();

    assert!(
        result.is_success(),
        "Result was unsuccessful {:?}",
        result.stdout()
    );
    assert!(
        result.stdout().contains("Dropping database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(
        result.stdout().contains("Creating database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
fn reset_works_with_migration_dir_by_arg() {
    let p = project("reset_works_with_migration_dir_by_arg")
        .folder("foo")
        .build();
    let db = database(&p.database_url()).create();

    db.execute("CREATE TABLE posts ( id INTEGER )");
    db.execute("CREATE TABLE users ( id INTEGER )");
    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    assert!(db.table_exists("posts"));
    assert!(db.table_exists("users"));

    let result = p
        .command("database")
        .arg("reset")
        .arg("--migration-dir=foo")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("posts"));
    assert!(db.table_exists("users"));
    assert!(db.table_exists("__diesel_schema_migrations"));
}

#[test]
fn reset_works_with_migration_dir_by_env() {
    let p = project("reset_works_with_migration_dir_by_env")
        .folder("bar")
        .build();
    let db = database(&p.database_url()).create();

    db.execute("CREATE TABLE posts ( id INTEGER )");
    db.execute("CREATE TABLE users ( id INTEGER )");
    p.create_migration_in_directory(
        "bar",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    assert!(db.table_exists("posts"));
    assert!(db.table_exists("users"));

    let result = p
        .command("database")
        .arg("reset")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("posts"));
    assert!(db.table_exists("users"));
    assert!(db.table_exists("__diesel_schema_migrations"));
}

#[test]
fn reset_sanitize_database_name() {
    let p = project("name-with-dashes").folder("migrations").build();
    let _db = database(&p.database_url()).create();

    let result = p.command("database").arg("reset").run();

    assert!(
        result.is_success(),
        "Result was unsuccessful {:?}",
        result.stdout()
    );
    assert!(
        result.stdout().contains("Dropping database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(
        result.stdout().contains("Creating database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
fn reset_updates_schema_if_config_present() {
    let p = project("reset_updates_schema_if_config_present")
        .folder("migrations")
        .file(
            "diesel.toml",
            r#"
            [print_schema]
            file = "src/my_schema.rs"
            "#,
        )
        .build();

    let result = p.command("database").arg("reset").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    assert!(p.has_file("src/my_schema.rs"));
}

#[test]
fn reset_respects_migrations_dir_from_diesel_toml() {
    let p = project("reset_respects_migrations_dir_from_diesel_toml")
        .folder("custom_migrations")
        .file(
            "diesel.toml",
            r#"
            [migrations_directory]
            dir = "custom_migrations"
            "#,
        )
        .build();
    let db = database(&p.database_url()).create();

    db.execute("CREATE TABLE users ( id INTEGER )");

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    assert!(db.table_exists("users"));

    let result = p.command("database").arg("reset").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("users"));
    assert!(db.table_exists("__diesel_schema_migrations"));
}
