use assert_cmd::Command;
use diesel::connection::SimpleConnection;
use diesel::{Connection, MysqlConnection};
use diesel_migrations::MigrationHarness;
use std::env;

struct DropGuard {
    conn: MysqlConnection,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.conn
            .batch_execute("DROP DATABASE getting_started_step_3")
            .unwrap();
    }
}

#[test]
fn show_posts() {
    let url = env::var("MYSQL_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL is set for tests");
    let mut db_url = url::Url::parse(&url).unwrap();

    db_url.set_path("getting_started_step_3");

    let mut conn = MysqlConnection::establish(&url).unwrap();
    conn.batch_execute("CREATE DATABASE getting_started_step_3")
        .unwrap();
    let _guard = DropGuard { conn };

    let mut conn = MysqlConnection::establish(db_url.as_ref()).unwrap();
    let migrations = diesel_migrations::FileBasedMigrations::find_migrations_directory().unwrap();
    conn.run_pending_migrations(migrations).unwrap();

    let _ = Command::cargo_bin("show_posts")
        .unwrap()
        .env("MYSQL_DATABASE_URL", db_url.to_string())
        .assert()
        .append_context("show_posts", "")
        .stdout("Displaying 0 posts\n");
}
