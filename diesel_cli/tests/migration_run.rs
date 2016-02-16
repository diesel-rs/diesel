use support::{database, project};

#[test]
fn migration_run_runs_pending_migrations() {
    let p = project("migration_run")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    assert!(!db.table_exists("users"));

    let result = p.command("migration")
        .arg("run")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}", result.stdout());
    assert!(db.table_exists("users"));
}
