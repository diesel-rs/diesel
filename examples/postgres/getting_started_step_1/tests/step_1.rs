use assert_cmd::Command;
use diesel::connection::SimpleConnection;
use diesel::{Connection, PgConnection};
use diesel_migrations::MigrationHarness;
use std::env;

struct DropGuard {
    conn: PgConnection,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.conn
            .batch_execute("DROP DATABASE getting_started_step_1")
            .unwrap();
    }
}

#[test]
fn show_posts() {
    let url = env::var("PG_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL is set for tests");
    let mut db_url = url::Url::parse(&url).unwrap();

    db_url.set_path("getting_started_step_1");

    let mut conn = PgConnection::establish(&url).unwrap();
    conn.batch_execute("CREATE DATABASE getting_started_step_1")
        .unwrap();
    let _guard = DropGuard { conn };

    let mut conn = PgConnection::establish(db_url.as_ref()).unwrap();
    let migrations = diesel_migrations::FileBasedMigrations::find_migrations_directory().unwrap();
    conn.run_pending_migrations(migrations).unwrap();

    let _ = Command::cargo_bin("show_posts")
        .unwrap()
        .env("PG_DATABASE_URL", db_url.to_string())
        .assert()
        .append_context("show_posts", "")
        .stdout("Displaying 0 posts\n");
}
