use support::project;

#[test]
fn migration_redo_runs_the_last_migration_down_and_up() {
    let p = project("migration_redo")
        .folder("migrations")
        .build();
    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

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
