use support::{database, project};
use diesel::{select, LoadDsl};
use diesel::expression::sql;
use diesel::types::Bool;

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

#[test]
fn migration_run_inserts_run_on_timestamps() {
    let p = project("migration_run_on_timestamps")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    // Make sure the project is setup.
    p.command("setup").run();

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    let migrations_done: bool = select(sql::<Bool>(
            "EXISTS (SELECT * FROM __diesel_schema_migrations)"))
        .get_result(&db.conn())
        .unwrap();
    assert!(!migrations_done, "Migrations table should be empty");

    let result = p.command("migration")
        .arg("run")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("users"));

    // By running a query that compares timestamps, we are also checking
    // that the auto-inserted values for the "run_on" column are valid.

    #[cfg(feature = "sqlite")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>("EXISTS (SELECT run_on < DATETIME('now', '-1 hour') \
                                    FROM __diesel_schema_migrations)"))
            .get_result(&db.conn())
            .unwrap()
    }

    #[cfg(feature = "postgres")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>("EXISTS (SELECT \
                              run_on < (CAST('now' AS TIMESTAMP) - \
                                        CAST('1 hour' AS INTERVAL)) \
                              FROM __diesel_schema_migrations)"))
            .get_result(&db.conn())
            .unwrap()
    }

    assert!(valid_run_on_timestamp(&db),
            "Running a migration did not insert an updated run_on value");
}
