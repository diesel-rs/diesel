use support::project;

#[test]
fn migration_redo_runs_the_last_migration_down_and_up() {
    let p = project("migration_redo").folder("migrations").build();
    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        "DROP TABLE users;",
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
fn migration_redo_respects_migration_dir_var() {
    let p = project("migration_redo_var").folder("foo").build();

    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        "DROP TABLE users;",
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
        "DROP TABLE users;",
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
fn output_contains_path_to_migration_script() {
    let p = project("output_contains_path_to_migration_script")
        .folder("migrations")
        .build();
    p.create_migration(
        "output_contains_path_to_migration_script",
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
        "DROP TABLE users};",
    );

    // Make sure the project is setup
    p.command("setup").run();

    let result = p.command("migration").arg("redo").run();

    assert!(!result.is_success(), "Result was successful {:?}", result);
    assert!(
        result.stdout().contains("down.sql"),
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
        "DROP TABLE users};",
    );

    // Make sure the project is setup
    p.command("setup").run();

    let result = p.command("migration").arg("redo").run();

    assert!(!result.is_success());
    assert!(result.stderr().contains("Failed with: "));
}
