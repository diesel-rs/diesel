use support::{database, project};
use diesel::{select, RunQueryDsl};
use diesel::dsl::sql;
use diesel::sql_types::Bool;

#[test]
fn migration_run_runs_pending_migrations() {
    let p = project("migration_run").folder("migrations").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
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
        "CREATE TABLE users ( id INTEGER )",
        "DROP TABLE users",
    );

    let migrations_done: bool = select(sql::<Bool>(
        "EXISTS (SELECT * FROM __diesel_schema_migrations WHERE version >= '1')",
    )).get_result(&db.conn())
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
        )).get_result(&db.conn())
            .unwrap()
    }

    #[cfg(feature = "postgres")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL '1 hour')",
        )).get_result(&db.conn())
            .unwrap()
    }

    #[cfg(feature = "mysql")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL 1 HOUR)",
        )).get_result(&db.conn())
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
    assert!(result.stdout().contains("empty migration"));
}

#[test]
fn any_pending_migrations_works() {
    let p = project("any_pending_migrations_one")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
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
        "CREATE TABLE users ( id INTEGER )",
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
        "CREATE TABLE users ( id INTEGER )",
        "DROP TABLE users",
    );

    p.command("migration").arg("run").run();

    p.create_migration(
        "123456_create_posts_table",
        "CREATE TABLE posts ( id INTEGER )",
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
        "CREATE TABLE users ( id INTEGER )",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command_without_database_url("migration")
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
        "CREATE TABLE users ( id INTEGER )",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command_without_database_url("migration")
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
        "CREATE TABLE users ( id INTEGER )",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration")
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
        "CREATE TABLE users ( id INTEGER )",
        "DROP TABLE users",
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration")
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
#[cfg(feature = "postgres")]
fn migrations_can_be_run_without_transactions() {
    let p = project("migration_run_without_transaction").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        "DROP TABLE users",
    );

    // CREATE INDEX CONCURRENTLY cannot be run inside a transaction.
    // It also can't be run in a multiline statement, so it needs its own
    // migration.
    p.create_migration(
        "12346_index_users_table",
        "CREATE UNIQUE INDEX CONCURRENTLY idx ON users (name)",
        "DROP INDEX idx",
    );
    p.add_migration_metadata("12346_index_users_table", "run_in_transaction = false");

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12346"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}
