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

#[test]
#[cfg(feature = "postgres")]
fn reset_handles_postgres_urls_with_username_and_password() {
    let p = project("handles_postgres_urls")
        .folder("migrations")
        .build();
    let db = database(&p.database_url()).create();

    db.execute("DROP ROLE IF EXISTS foo");
    db.execute("CREATE ROLE foo WITH LOGIN SUPERUSER PASSWORD 'password'");

    let database_url = format!("postgres://foo:password@localhost/diesel_{}", p.name);
    let result = p.command("database")
        .arg("reset")
        .env("DATABASE_URL", &database_url)
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result.stdout());
}
