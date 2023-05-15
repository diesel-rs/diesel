use crate::support::{database, project};

#[test]
fn migration_redo_runs_the_last_migration_down_and_up() {
    let p = project("migration_redo").folder("migrations").build();
    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        Some("DROP TABLE users;"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    let result = p.command("migration").arg("redo").run();

    let expected_stdout = "\
Rolling back migration 12345_create_users_table
Running migration 12345_create_users_table
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains(expected_stdout),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
fn migration_redo_runs_the_last_two_migrations_down_and_up() {
    let p = project("migration_redo_runs_the_last_two_migrations_down_and_up")
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

    // Redo the last two migration files. The `contracts` and `bills` tables should be re-runs.
    // The `customers` table shouldn't be redo.
    let result = p.command("migration").arg("redo").arg("-n").arg("2").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout()
            == "Rolling back migration 2017-09-12-210424_create_bills\n\
                Rolling back migration 2017-09-03-210424_create_contracts\n\
                Running migration 2017-09-03-210424_create_contracts\n\
                Running migration 2017-09-12-210424_create_bills\n",
        "Unexpected stdout : {}",
        result.stdout()
    );

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("contracts"));
    assert!(db.table_exists("bills"));
}

#[test]
fn migration_redo_respects_migration_dir_var() {
    let p = project("migration_redo_var").folder("foo").build();

    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        Some("DROP TABLE users;"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").arg("--migration-dir=foo").run();

    let result = p
        .command("migration")
        .arg("redo")
        .arg("--migration-dir=foo")
        .run();

    let expected_stdout = "\
Rolling back migration 12345_create_users_table
Running migration 12345_create_users_table
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains(expected_stdout),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
fn migration_redo_respects_migration_dir_env() {
    let p = project("migration_redo_env").folder("bar").build();

    p.create_migration_in_directory(
        "bar",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        Some("DROP TABLE users;"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").arg("--migration-dir=bar").run();

    let result = p
        .command("migration")
        .arg("redo")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    let expected_stdout = "\
Rolling back migration 12345_create_users_table
Running migration 12345_create_users_table
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains(expected_stdout),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
fn error_migrations_fails() {
    let p = project("redo_error_migrations_fails")
        .folder("migrations")
        .build();
    p.create_migration(
        "redo_error_migrations_fails",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        Some("DROP TABLE users};"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    let result = p.command("migration").arg("redo").run();

    assert!(!result.is_success());
    assert!(result
        .stderr()
        .contains("Failed to run redo_error_migrations_fails with: "));
}

#[test]
fn migration_redo_respects_migrations_dir_from_diesel_toml() {
    let p = project("migration_redo_respects_migrations_dir_from_diesel_toml")
        .folder("custom_migrations")
        .file(
            "diesel.toml",
            r#"
            [migrations_directory]
            dir = "custom_migrations"
            "#,
        )
        .build();

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        Some("DROP TABLE users;"),
        None,
    );

    // Make sure the project is setup
    p.command("setup").run();

    let result = p.command("migration").arg("redo").run();

    let expected_stdout = "\
Rolling back migration 12345_create_users_table
Running migration 12345_create_users_table
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains(expected_stdout),
        "Unexpected stdout {}",
        result.stdout()
    );
}

#[test]
fn migration_redo_all_runs_all_migrations_down_and_up() {
    let p = project("migration_redo_all_runs_all_migrations_down_and_up")
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

    let result = p.command("migration").arg("redo").arg("--all").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    assert!(
        result.stdout()
            == "Rolling back migration 2017-09-12-210424_create_bills\n\
                Rolling back migration 2017-09-03-210424_create_contracts\n\
                Rolling back migration 2017-08-31-210424_create_customers\n\
                Running migration 2017-08-31-210424_create_customers\n\
                Running migration 2017-09-03-210424_create_contracts\n\
                Running migration 2017-09-12-210424_create_bills\n",
        "Unexpected stdout : {}",
        result.stdout()
    );

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("contracts"));
    assert!(db.table_exists("bills"));
}

#[test]
fn migration_redo_with_more_than_max_should_redo_all() {
    let p = project("migration_redo_with_more_than_max_should_redo_all")
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
        .arg("redo")
        .arg("-n")
        .arg("1000")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    assert!(
        result.stdout()
            == "Rolling back migration 2017-09-12-210424_create_bills\n\
                Rolling back migration 2017-09-03-210424_create_contracts\n\
                Rolling back migration 2017-08-31-210424_create_customers\n\
                Running migration 2017-08-31-210424_create_customers\n\
                Running migration 2017-09-03-210424_create_contracts\n\
                Running migration 2017-09-12-210424_create_bills\n",
        "Unexpected stdout : {}",
        result.stdout()
    );

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("contracts"));
    assert!(db.table_exists("bills"));
}

#[test]
fn migration_redo_n_with_a_string_should_throw_an_error() {
    let p = project("migration_redo_n_with_a_string_should_throw_an_error")
        .folder("migrations")
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    // Should not revert any migration.
    let result = p
        .command("migration")
        .arg("redo")
        .arg("-n")
        .arg("infinite")
        .run();

    assert!(!result.is_success(), "Result was unsuccessful {:?}", result);

    assert!(
        result.stderr()
            == "error: invalid value 'infinite' for '--number <REDO_NUMBER>': \
                invalid digit found in string\n\n\
                For more information, try '--help'.\n",
        "Unexpected stderr : '{}'",
        result.stderr()
    );
}

#[test]
fn migration_redo_with_zero_should_not_revert_any_migration() {
    let p = project("migration_redo_with_zero_should_not_revert_any_migration")
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
    let result = p.command("migration").arg("redo").arg("-n").arg("0").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout() == "");
}

#[cfg(not(feature = "mysql"))] // mysql does not support DDL + Transactions
#[test]
fn migration_redo_without_transaction() {
    let p = project("migration_redo_without_transaction")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2023-05-08-210424_without_transaction",
        "BEGIN TRANSACTION;CREATE TABLE customers ( id INTEGER PRIMARY KEY );COMMIT TRANSACTION;",
        Some("BEGIN TRANSACTION;DROP TABLE customers; COMMIT TRANSACTION;"),
        Some("run_in_transaction = false"),
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));

    // Should not revert any migration.
    let result = p.command("migration").arg("redo").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout()
            == "Rolling back migration 2023-05-08-210424_without_transaction\n\
                                Running migration 2023-05-08-210424_without_transaction\n"
    );
    assert!(db.table_exists("customers"));
}

#[cfg(not(feature = "mysql"))] // mysql does not support DDL + Transactions
#[test]
fn migration_redo_without_transaction_twice() {
    let p = project("migration_redo_without_transaction_twice")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2023-05-08-210424_without_transaction",
        "BEGIN TRANSACTION;CREATE TABLE customers ( id INTEGER PRIMARY KEY );COMMIT TRANSACTION;",
        Some("BEGIN TRANSACTION;DROP TABLE customers; COMMIT TRANSACTION;"),
        Some("run_in_transaction = false"),
    );

    p.create_migration(
        "2023-05-08-210425_without_transaction2",
        "BEGIN TRANSACTION;CREATE TABLE customers2 ( id INTEGER PRIMARY KEY );COMMIT TRANSACTION;",
        Some("BEGIN TRANSACTION;DROP TABLE customers2; COMMIT TRANSACTION;"),
        Some("run_in_transaction = false"),
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("customers2"));

    // Should not revert any migration.
    let result = p.command("migration").arg("redo").arg("-n").arg("2").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout()
            == "Rolling back migration 2023-05-08-210425_without_transaction2\n\
                Rolling back migration 2023-05-08-210424_without_transaction\n\
                Running migration 2023-05-08-210424_without_transaction\n\
                Running migration 2023-05-08-210425_without_transaction2\n"
    );
    assert!(db.table_exists("customers"));
    assert!(db.table_exists("customers2"));
}

#[cfg(not(feature = "mysql"))] // mysql does not support DDL + Transactions
#[test]
fn migration_redo_without_transaction_all() {
    let p = project("migration_redo_without_transaction_all")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration(
        "2023-05-08-210424_without_transaction",
        "BEGIN TRANSACTION;CREATE TABLE customers ( id INTEGER PRIMARY KEY );COMMIT TRANSACTION;",
        Some("BEGIN TRANSACTION;DROP TABLE customers; COMMIT TRANSACTION;"),
        Some("run_in_transaction = false"),
    );

    p.create_migration(
        "2023-05-08-210425_without_transaction2",
        "BEGIN TRANSACTION;CREATE TABLE customers2 ( id INTEGER PRIMARY KEY );COMMIT TRANSACTION;",
        Some("BEGIN TRANSACTION;DROP TABLE customers2; COMMIT TRANSACTION;"),
        Some("run_in_transaction = false"),
    );

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("customers"));
    assert!(db.table_exists("customers2"));

    // Should not revert any migration.
    let result = p.command("migration").arg("redo").arg("--all").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout()
            == "Rolling back migration 2023-05-08-210425_without_transaction2\n\
                Rolling back migration 2023-05-08-210424_without_transaction\n\
                Running migration 2023-05-08-210424_without_transaction\n\
                Running migration 2023-05-08-210425_without_transaction2\n"
    );
    assert!(db.table_exists("customers"));
    assert!(db.table_exists("customers2"));
}
