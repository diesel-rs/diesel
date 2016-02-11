use regex::Regex;

use support::project;

#[test]
fn migration_generate_creates_a_migration_with_the_proper_name() {
    let p = project("migration_name")
        .folder("migrations")
        .build();
    let result = p.command("migration")
        .arg("generate")
        .arg("hello")
        .run();

    let expected_stdout = Regex::new("\
Creating migrations/\\d{14}_hello/up.sql
Creating migrations/\\d{14}_hello/down.sql\
        ").unwrap();
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert!(expected_stdout.is_match(result.stdout()));

    let migrations = p.migrations();
    assert_eq!(1, migrations.len());

    let migration = &migrations[0];
    assert_eq!("hello", migration.name());
    assert!(migration.path().join("up.sql").exists());
    assert!(migration.path().join("down.sql").exists());
}

#[test]
fn migration_generate_doesnt_require_database_url_to_be_set() {
    let p = project("migration_name")
        .folder("migrations")
        .build();
    let result = p.command_without_datatabase_url("migration")
        .arg("generate")
        .arg("hello")
        .run();

    assert!(result.is_success(), "Command failed: {:?}", result);
}

#[test]
fn migration_version_can_be_specified_on_creation() {
    let p = project("migration_name")
        .folder("migrations")
        .build();
    let result = p.command("migration")
        .arg("generate")
        .arg("hello")
        .arg("--version=1234")
        .run();

    let expected_stdout = "\
Creating migrations/1234_hello/up.sql
Creating migrations/1234_hello/down.sql
";
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert_eq!(expected_stdout, result.stdout());

    assert!(p.has_file("migrations/1234_hello/up.sql"));
    assert!(p.has_file("migrations/1234_hello/down.sql"));
}

#[test]
fn migration_directory_can_be_specified_for_generate_by_command_line_arg() {
    let p = project("migration_name")
        .folder("foo")
        .build();
    let result = p.command("migration")
        .arg("generate")
        .arg("stuff")
        .arg("--version=12345")
        .arg("--migration-dir=foo")
        .run();

    let expected_stdout = "\
Creating foo/12345_stuff/up.sql
Creating foo/12345_stuff/down.sql
";
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert_eq!(expected_stdout, result.stdout());

    assert!(p.has_file("foo/12345_stuff/up.sql"));
    assert!(p.has_file("foo/12345_stuff/down.sql"));
}

#[test]
fn migration_directory_can_be_specified_for_generate_by_env_var() {
    let p = project("migration_name")
        .folder("bar")
        .build();
    let result = p.command("migration")
        .arg("generate")
        .arg("stuff")
        .arg("--version=12345")
        .env("MIGRATION_DIRECTORY", "bar")
        .run();

    let expected_stdout = "\
Creating bar/12345_stuff/up.sql
Creating bar/12345_stuff/down.sql
";
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert_eq!(expected_stdout, result.stdout());

    assert!(p.has_file("bar/12345_stuff/up.sql"));
    assert!(p.has_file("bar/12345_stuff/down.sql"));
}
