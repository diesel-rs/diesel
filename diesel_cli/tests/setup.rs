extern crate diesel_test_helpers;
extern crate diesel;

use self::diesel::Connection;

use self::diesel_test_helpers::{TestDatabase, TestEnvironment, table_exists, TestConnection};

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn build_diesel() {
    Command::new("cargo").arg("build").output().unwrap();
}

fn diesel_exe() -> PathBuf {
    env::current_exe().unwrap().parent().unwrap().join("diesel")
}

#[test]
fn diesel_setup_creates_database_and_migrations_dir() {
    build_diesel();
    let test_environment = TestEnvironment::new();
    let test_database = TestDatabase::new(&test_environment.identifier, &test_environment.root_path());
    fs::File::create(&test_environment.root_path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(diesel_exe());
    command.arg("setup")
        .env("DATABASE_URL", &test_database.database_url)
        .current_dir(&test_environment.root_path());
    let output = command.output().unwrap();
    assert_eq!(format!("Creating migrations/ directory at: {}\n", &test_environment.root_path().join("migrations").display()),
                    String::from_utf8(output.stdout).unwrap());
    assert!(table_exists(&test_database.database_url, "__diesel_schema_migrations"));
    assert!(fs::metadata(&test_environment.root_path().join("migrations")).is_ok());
}

#[test]
fn diesel_setup_runs_existing_migrations_if_no_schema_table() {
    build_diesel();
    let test_environment = TestEnvironment::new();
    let test_database = TestDatabase::new(&test_environment.identifier, &test_environment.root_path());
    let migrations_dir = test_environment.root_path().join("migrations");
    fs::create_dir(&migrations_dir).unwrap();

    let create_users_dir = migrations_dir.join("12345_create_users_table");
    fs::create_dir(&create_users_dir).unwrap();

    let mut up_sql = fs::File::create(&create_users_dir.join("up.sql")).unwrap();
    up_sql.write_all(b"CREATE TABLE users ( id INTEGER )").unwrap();

    let mut down_sql = fs::File::create(&create_users_dir.join("down.sql")).unwrap();
    down_sql.write_all(b"DROP TABLE users").unwrap();

    let mut command = Command::new(diesel_exe());
    command.arg("setup")
        .env("DATABASE_URL", &test_database.database_url)
        .current_dir(&test_environment.root_path());
    let output = command.output().unwrap();

    assert_eq!("Running migration 12345\n".as_bytes().to_vec(), output.stdout);
    assert!(table_exists(&test_database.database_url, "users"));
}

#[test]
fn diesel_setup_doesnt_run_migrations_if_schema_table_exists() {
    build_diesel();
    let test_environment = TestEnvironment::new();
    let test_database = TestDatabase::new(&test_environment.identifier, &test_environment.root_path());
    let migrations_dir = test_environment.root_path().join("migrations");
    fs::create_dir(&migrations_dir).unwrap();

    let create_users_dir = migrations_dir.join("12345_create_users_table");
    fs::create_dir(&create_users_dir).unwrap();

    let mut up_sql = fs::File::create(&create_users_dir.join("up.sql")).unwrap();
    up_sql.write_all(b"CREATE TABLE users ( id INTEGER )").unwrap();

    let mut down_sql = fs::File::create(&create_users_dir.join("down.sql")).unwrap();
    down_sql.write_all(b"DROP TABLE users").unwrap();

    let connection = TestConnection::establish(&test_database.database_url).unwrap();
    connection.execute("CREATE TABLE __diesel_schema_migrations ( version INTEGER )").unwrap();

    let mut command = Command::new(diesel_exe());
    command.arg("setup")
        .env("DATABASE_URL", &test_database.database_url)
        .current_dir(&test_environment.root_path());
    let output = command.output().unwrap();

    assert_eq!("".as_bytes().to_vec(), output.stdout);
    assert!(!table_exists(&test_database.database_url, "users"));
}
