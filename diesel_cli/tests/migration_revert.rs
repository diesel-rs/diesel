use support::{database, project};

#[test]
fn migration_revert_runs_the_last_migration_down() {
    let p = project("migration_revert")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    // Make sure the project is setup
    p.command("setup").run();

    assert!(db.table_exists("users"));

    let result = p.command("migration")
        .arg("revert")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Rolling back migration 12345"),
        "Unexpected stdout {}", result.stdout());
    assert!(!db.table_exists("users"));
}
