extern crate diesel_test_helpers;

use self::diesel_test_helpers::{TestDatabase, TestEnvironment, table_exists};
use std::process::Command;
use std::{env, fs};

#[test]
fn diesel_setup() {
    let test_environment = TestEnvironment::new();
    let test_database = TestDatabase::new(&test_environment.identifier, &test_environment.root_path());
    Command::new("cargo").arg("build").output().unwrap();
    let diesel_exe = env::current_exe().unwrap().parent().unwrap().join("diesel");
    let mut command = Command::new(diesel_exe);
    fs::File::create(&test_environment.root_path().join("Cargo.toml")).unwrap();
    command.arg("setup")
        .env("DATABASE_URL", &test_database.database_url)
        .current_dir(&test_environment.root_path());
    let output = command.output().unwrap();
    assert_eq!(format!("Creating migrations/ directory at: {}\n", &test_environment.root_path().join("migrations").display()),
                    String::from_utf8(output.stdout).unwrap());
    assert!(table_exists(&test_database.database_url, "__diesel_schema_migrations"));
    assert!(fs::metadata(&test_environment.root_path().join("migrations")).is_ok());
}
