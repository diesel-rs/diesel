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
            .batch_execute("DROP DATABASE getting_started_step_2")
            .unwrap();
    }
}

#[test]
fn write_post() {
    let url = env::var("PG_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL is set for tests");
    let mut db_url = url::Url::parse(&url).unwrap();

    db_url.set_path("getting_started_step_2");

    let mut conn = PgConnection::establish(&url).unwrap();
    conn.batch_execute("CREATE DATABASE getting_started_step_2")
        .unwrap();
    let _guard = DropGuard { conn };

    let mut conn = PgConnection::establish(db_url.as_ref()).unwrap();
    let migrations = diesel_migrations::FileBasedMigrations::find_migrations_directory().unwrap();
    conn.run_pending_migrations(migrations).unwrap();

    let _ = Command::cargo_bin("write_post")
        .unwrap()
        .env("PG_DATABASE_URL", db_url.to_string())
        .write_stdin("Test Title\ntest text\n1 2 3")
        .assert()
        .append_context("write_post", "")
        .stdout(
            "What would you like your title to be?\n\nOk! Let's write Test Title (Press "
                .to_owned()
                + EOF
                + " when finished)\n\n\nSaved draft Test Title with id 1\n",
        );
    let _ = Command::cargo_bin("show_posts")
        .unwrap()
        .env("PG_DATABASE_URL", db_url.to_string())
        .assert()
        .append_context("show_posts", "")
        .stdout("Displaying 0 posts\n");
}

#[cfg(not(windows))]
const EOF: &str = "CTRL+D";

#[cfg(windows)]
const EOF: &str = "CTRL+Z";
