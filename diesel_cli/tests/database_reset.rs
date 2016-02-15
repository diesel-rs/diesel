use support::{database, project};

#[test]
fn reset_drops_the_database() {
    let p = project("reset_drops_the_database")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();
    db.execute("CREATE TABLE posts ( id INTEGER )");

    assert!(db.table_exists("posts"));

    let result = p.command("database")
        .arg("reset")
        .run();

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
    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    assert!(db.table_exists("posts"));
    assert!(db.table_exists("users"));

    let result = p.command("database")
        .arg("reset")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(!db.table_exists("posts"));
    assert!(db.table_exists("users"));
    assert!(db.table_exists("__diesel_schema_migrations"));
}
