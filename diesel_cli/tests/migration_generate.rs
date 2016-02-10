extern crate diesel_test_helpers;

use self::diesel_test_helpers::{TestEnvironment};

use std::process::Command;
use std::path::PathBuf;
use std::{env, fs};

#[test]
fn diesel_migration_generate_makes_valid_migration_directory() {
    Command::new("cargo").arg("build").output().unwrap();

    let test_environment = TestEnvironment::new();

    let migrations_dir = test_environment.root_path().join("migrations");
    fs::create_dir(&migrations_dir).unwrap();

    let diesel_exe = env::current_exe().unwrap().parent().unwrap().join("diesel");
    let mut command = Command::new(diesel_exe);
    command.arg("migration")
        .arg("generate")
        .arg("create_posts_table")
        .current_dir(&test_environment.root_path());
    command.output().unwrap();

    let create_posts_dir = fs::read_dir(&migrations_dir).unwrap().nth(0).unwrap().unwrap().path();
    let filenames: Vec<String> = create_posts_dir.read_dir()
        .unwrap().map(|entry| {
            entry.unwrap().file_name().into_string().unwrap()
        }).collect();
    assert_eq!(vec!["down.sql".to_owned(), "up.sql".to_owned()], filenames);
}
