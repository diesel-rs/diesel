use crate::support::{database, project};

#[test]
fn migration_revert_runs_the_last_migration_down() {
    let p = project("migration_revert_runs_the_last_migration_down")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("users"));

    let result = p.command("migration").arg("revert").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Rolling back migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(!db.table_exists("users"));
}

#[test]
fn migration_revert_respects_migration_dir_var() {
    let p = project("migration_revert_var").folder("foo").build();
    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // Make sure the project is setup.
    p.command("setup").arg("--migration-dir=foo").run();

    assert!(db.table_exists("users"));

    let result = p
        .command("migration")
        .arg("revert")
        .arg("--migration-dir=foo")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Rolling back migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(!db.table_exists("users"));
}

#[test]
fn migration_revert_respects_migration_dir_env() {
    let p = project("migration_revert_env").folder("bar").build();
    let db = database(&p.database_url());

    p.create_migration_in_directory(
        "bar",
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        Some("DROP TABLE users"),
        None,
    );

    // Make sure the project is setup.
    p.command("setup").arg("--migration-dir=bar").run();

    assert!(db.table_exists("users"));

    let result = p
        .command("migration")
        .arg("revert")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Rolling back migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(!db.table_exists("users"));
}

#[test]
fn migration_revert_respects_migration_dir_from_diesel_toml() {
    let p = project("migration_revert_respects_migration_dir_from_diesel_toml")
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

    // Make sure the project is setup.
    p.command("setup").run();

    assert!(db.table_exists("users"));

    let result = p.command("migration").arg("revert").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Rolling back migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(!db.table_exists("users"));
}

#[test]
fn migration_revert_runs_the_last_two_migration_down() {
    let p = project("migration_revert_runs_the_last_two_migration_down")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2017-08-31-210424_create_customers",
        "CREATE TABLE customers ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE customers"),
        None,
    );

    p.create_migration(
        "2017-09-03-210424_create_contracts",
        "CREATE TABLE contracts ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE contracts"),
        None,
    );

    p.create_migration(
        "2017-09-12-210424_create_bills",
        "CREATE TABLE bills ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE bills"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("contracts"));
    assert!(db.table_exists("bills"));

    // Reverts the last two migration files. The `contracts` and `bills` tables should be dropped.
    // The `customers` table shouldn't be reverted.
    let result = p
        .command("migration")
        .arg("revert")
        .arg("-n")
        .arg("2")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Rolling back migration 2017-09-12-210424_create_bills\nRolling back migration 2017-09-03-210424_create_contracts"),
        "Unexpected stdout {}",
        result.stdout()
    );

    assert!(db.table_exists("customers"));
    assert!(!db.table_exists("contracts"));
    assert!(!db.table_exists("bills"));
}

#[test]
fn migration_revert_all_runs_the_migrations_down() {
    let p = project("migration_revert_all_runs_the_migrations_down")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2017-08-31-210424_create_customers",
        "CREATE TABLE customers ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE customers"),
        None,
    );

    p.create_migration(
        "2017-09-03-210424_create_contracts",
        "CREATE TABLE contracts ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE contracts"),
        None,
    );

    p.create_migration(
        "2017-09-12-210424_create_bills",
        "CREATE TABLE bills ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE bills"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("contracts"));
    assert!(db.table_exists("bills"));

    let result = p.command("migration").arg("revert").arg("--all").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    assert!(
        result.stdout()
            == "Rolling back migration 2017-09-12-210424_create_bills\n\
                Rolling back migration 2017-09-03-210424_create_contracts\n\
                Rolling back migration 2017-08-31-210424_create_customers\n",
        "Unexpected stdout : {}",
        result.stdout()
    );

    assert!(!db.table_exists("customers"));
    assert!(!db.table_exists("contracts"));
    assert!(!db.table_exists("bills"));
}

#[test]
fn migration_revert_with_zero_should_not_revert_any_migration() {
    let p = project("migration_revert_with_zero_should_not_revert")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2017-08-31-210424_create_customers",
        "CREATE TABLE customers ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE customers"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));

    // Should not revert any migration.
    let result = p
        .command("migration")
        .arg("revert")
        .arg("-n")
        .arg("0")
        .run();
    assert!(
        result.is_success(),
        "Result was unsuccessful '{:?}'",
        result
    );
    assert!(result.stdout() == "");
}

#[test]
fn migration_revert_n_with_a_string_should_throw_an_error() {
    let p = project("migration_revert_with_an_invalid_input_should_throw_an_error")
        .folder("migrations")
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    // Should not revert any migration.
    let result = p
        .command("migration")
        .arg("revert")
        .arg("-n")
        .arg("infinite")
        .run();

    assert!(!result.is_success(), "Result was successful {:?}", result);

    assert!(
        result.stderr()
            == "error: invalid value 'infinite' for '--number <REVERT_NUMBER>': \
                invalid digit found in string\n\n\
                For more information, try '--help'.\n",
        "Unexpected stderr : {}",
        result.stderr()
    );
}

#[test]
fn migration_revert_with_more_than_max_should_revert_all() {
    let p = project("migration_revert_with_more_than_max_should_revert_all")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2017-08-31-210424_create_customers",
        "CREATE TABLE customers ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE customers"),
        None,
    );

    p.create_migration(
        "2017-09-03-210424_create_contracts",
        "CREATE TABLE contracts ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE contracts"),
        None,
    );

    p.create_migration(
        "2017-09-12-210424_create_bills",
        "CREATE TABLE bills ( id INTEGER PRIMARY KEY )",
        Some("DROP TABLE bills"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("contracts"));
    assert!(db.table_exists("bills"));

    let result = p
        .command("migration")
        .arg("revert")
        .arg("-n")
        .arg("1000")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    assert!(
        result.stdout()
            == "Rolling back migration 2017-09-12-210424_create_bills\n\
                Rolling back migration 2017-09-03-210424_create_contracts\n\
                Rolling back migration 2017-08-31-210424_create_customers\n",
        "Unexpected stdout : {}",
        result.stdout()
    );

    assert!(!db.table_exists("customers"));
    assert!(!db.table_exists("contracts"));
    assert!(!db.table_exists("bills"));
}

#[test]
fn migration_revert_gives_reasonable_error_message_on_missing_down() {
    let p = project("migration_revert_error_message_on_missing_down")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users ( id INTEGER )",
        None,
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("users"));

    let result = p.command("migration").arg("revert").run();

    assert!(
        !result.is_success(),
        "Result was successful when it shouldn't be {:?}",
        result
    );
    assert!(
        result.stdout().contains("Rolling back migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(
        result
            .stderr()
            .contains("Missing `down.sql` file to revert migration"),
        "Unexpected stderr {}",
        result.stderr()
    );
    assert!(db.table_exists("users"));
}
