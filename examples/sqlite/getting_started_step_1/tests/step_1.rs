use assert_cmd::Command;
use diesel::Connection;
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;

#[test]
fn show_posts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_url = temp_dir.path().join("test.db").display().to_string();
    let mut conn = SqliteConnection::establish(&db_url).unwrap();
    let migrations = diesel_migrations::FileBasedMigrations::find_migrations_directory().unwrap();
    conn.run_pending_migrations(migrations).unwrap();

    let _ = Command::cargo_bin("show_posts")
        .unwrap()
        .env("SQLITE_DATABASE_URL", &db_url)
        .assert()
        .append_context("show_posts", "")
        .stdout("Displaying 0 posts\n");
}
