use chrono::prelude::*;
use regex::Regex;

use migrations_internals::TIMESTAMP_FORMAT;
use support::project;

#[test]
fn migration_generate_creates_a_migration_with_the_proper_name() {
    let p = project("migration_name").folder("migrations").build();
    let result = p.command("migration").arg("generate").arg("hello").run();

    // check overall output
    let expected_stdout = Regex::new(
        "\
Creating migrations.\\d{4}-\\d{2}-\\d{2}-\\d{6}_hello.up.sql
Creating migrations.\\d{4}-\\d{2}-\\d{2}-\\d{6}_hello.down.sql\
        ",
    )
    .unwrap();
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert!(expected_stdout.is_match(result.stdout()));

    // check timestamps are properly formatted
    let captured_timestamps = Regex::new(r"(?P<stamp>[\d-]*)_hello").unwrap();
    let mut stamps_found = 0;
    for caps in captured_timestamps.captures_iter(result.stdout()) {
        let timestamp = Utc.datetime_from_str(&caps["stamp"], TIMESTAMP_FORMAT);
        assert!(
            timestamp.is_ok(),
            "Found invalid timestamp format: {:?}",
            &caps["stamp"]
        );
        stamps_found += 1;
    }
    assert_eq!(stamps_found, 2);

    let migrations = p.migrations();
    assert_eq!(1, migrations.len());

    let migration = &migrations[0];
    assert_eq!("hello", migration.name());
    assert!(migration.path().join("up.sql").exists());
    assert!(migration.path().join("down.sql").exists());
}

#[test]
fn migration_generate_creates_a_migration_with_initial_contents() {
    let p = project("migration_name").folder("migrations").build();
    let result = p.command("migration").arg("generate").arg("hello").run();
    assert!(result.is_success(), "Command failed: {:?}", result);

    let migrations = p.migrations();
    let migration = &migrations[0];

    let up = file_content(&migration.path().join("up.sql"));
    let down = file_content(&migration.path().join("down.sql"));

    assert_eq!(up.trim(), "-- Your SQL goes here");
    assert_eq!(down.trim(), "-- This file should undo anything in `up.sql`");

    fn file_content<P: AsRef<::std::path::Path>>(path: P) -> String {
        use std::io::Read;

        let mut file = ::std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        contents
    }
}

#[test]
fn migration_generate_doesnt_require_database_url_to_be_set() {
    let p = project("migration_name").folder("migrations").build();
    let result = p
        .command_without_database_url("migration")
        .arg("generate")
        .arg("hello")
        .run();

    assert!(result.is_success(), "Command failed: {:?}", result);
}

#[test]
fn migration_version_can_be_specified_on_creation() {
    let p = project("migration_name").folder("migrations").build();
    let result = p
        .command("migration")
        .arg("generate")
        .arg("hello")
        .arg("--version=1234")
        .run();

    let expected_stdout = Regex::new(
        "\
Creating migrations.1234_hello.up.sql
Creating migrations.1234_hello.down.sql
",
    )
    .unwrap();
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert!(expected_stdout.is_match(result.stdout()));

    assert!(p.has_file("migrations/1234_hello/up.sql"));
    assert!(p.has_file("migrations/1234_hello/down.sql"));
}

#[test]
fn migration_directory_can_be_specified_for_generate_by_command_line_arg() {
    let p = project("migration_name").folder("foo").build();
    let result = p
        .command("migration")
        .arg("generate")
        .arg("stuff")
        .arg("--version=12345")
        .arg("--migration-dir=foo")
        .run();

    let expected_stdout = Regex::new(
        "\
Creating foo.12345_stuff.up.sql
Creating foo.12345_stuff.down.sql
",
    )
    .unwrap();
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert!(expected_stdout.is_match(result.stdout()));

    assert!(p.has_file("foo/12345_stuff/up.sql"));
    assert!(p.has_file("foo/12345_stuff/down.sql"));
}

#[test]
fn migration_directory_can_be_specified_for_generate_by_env_var() {
    let p = project("migration_name").folder("bar").build();
    let result = p
        .command("migration")
        .arg("generate")
        .arg("stuff")
        .arg("--version=12345")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    let expected_stdout = Regex::new(
        "\
Creating bar.12345_stuff.up.sql
Creating bar.12345_stuff.down.sql
",
    )
    .unwrap();
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert!(expected_stdout.is_match(result.stdout()));

    assert!(p.has_file("bar/12345_stuff/up.sql"));
    assert!(p.has_file("bar/12345_stuff/down.sql"));
}
