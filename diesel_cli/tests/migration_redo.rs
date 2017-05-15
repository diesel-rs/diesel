use support::project;

#[test]
fn migration_redo_runs_the_last_migration_down_and_up() {
    let p = project("migration_redo")
        .folder("migrations")
        .build();
    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users (id INTEGER);",
                       "DROP TABLE users;");

    // Make sure the project is setup
    p.command("setup").run();

    let result = p.command("migration")
        .arg("redo")
        .run();

    let expected_stdout = "\
Rolling back migration 12345
Running migration 12345
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains(expected_stdout),
        "Unexpected stdout {}", result.stdout());
}

#[test]
fn migration_redo_respects_migration_dir_var() {
    let p = project("migration_redo_var")
        .folder("foo")
        .build();

    p.create_migration_in_directory(
        "foo",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER);",
        "DROP TABLE users;"
    );

    // Make sure the project is setup
    p.command("setup")
        .arg("--migration-dir=foo")
        .run();

    let result = p.command("migration")
        .arg("redo")
        .arg("--migration-dir=foo")
        .run();

    let expected_stdout = "\
Rolling back migration 12345
Running migration 12345
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains(expected_stdout),
        "Unexpected stdout {}", result.stdout());
}

#[test]
fn migration_redo_respects_migration_dir_env() {
    let p = project("migration_redo_env")
        .folder("bar")
        .build();

    p.create_migration_in_directory(
        "bar",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER);",
        "DROP TABLE users;"
    );

    // Make sure the project is setup
    p.command("setup")
        .arg("--migration-dir=bar")
        .run();

    let result = p.command("migration")
        .arg("redo")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    let expected_stdout = "\
Rolling back migration 12345
Running migration 12345
";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains(expected_stdout),
        "Unexpected stdout {}", result.stdout());
}
