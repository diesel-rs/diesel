use std::path::{Path, PathBuf};
use std::{fs::File, io::Read};

use chrono::prelude::*;
use regex::Regex;

use crate::support::{project, Project};
pub static TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

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
        let timestamp = NaiveDateTime::parse_from_str(&caps["stamp"], TIMESTAMP_FORMAT);
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

    let up = file_content(migration.path().join("up.sql"));
    let down = file_content(migration.path().join("down.sql"));

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
fn migration_generate_with_no_down_file_has_no_down_file() {
    let p = project("migration_name").folder("migrations").build();
    let result = p
        .command("migration")
        .arg("generate")
        .arg("--no-down")
        .arg("hello")
        .run();
    assert!(result.is_success(), "Command failed: {:?}", result);

    let migrations = p.migrations();
    let migration = &migrations[0];

    assert!(migration.path().join("up.sql").exists());
    assert!(!migration.path().join("down.sql").exists());
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

#[test]
fn migration_generate_respects_migrations_dir_from_diesel_toml() {
    let p = project("migration_name")
        .folder("custom_migrations")
        .file(
            "diesel.toml",
            r#"
            [migrations_directory]
            dir = "custom_migrations"
            "#,
        )
        .build();

    let result = p
        .command("migration")
        .arg("generate")
        .arg("stuff")
        .arg("--version=12345")
        .run();

    let expected_stdout = Regex::new(
        "\
Creating custom_migrations.12345_stuff.up.sql
Creating custom_migrations.12345_stuff.down.sql
",
    )
    .unwrap();
    assert!(result.is_success(), "Command failed: {:?}", result);
    assert!(expected_stdout.is_match(result.stdout()));

    assert!(p.has_file("custom_migrations/12345_stuff/up.sql"));
    assert!(p.has_file("custom_migrations/12345_stuff/down.sql"));
}

#[test]
fn migration_generate_from_diff_drop_table() {
    test_generate_migration("diff_drop_table", Vec::new());
}

#[test]
fn migration_generate_from_diff_add_table() {
    test_generate_migration("diff_add_table", Vec::new());
}

#[test]
fn migration_generate_from_diff_add_table_sqlite_rowid_column() {
    test_generate_migration(
        "diff_add_table_sqlite_rowid_column",
        vec!["--sqlite-integer-primary-key-is-bigint"],
    );
}

#[test]
fn migration_generate_from_diff_drop_alter_table_add_column() {
    test_generate_migration("diff_alter_table_add_column", Vec::new());
}

#[test]
fn migration_generate_from_diff_alter_table_drop_column() {
    test_generate_migration("diff_alter_table_drop_column", Vec::new());
}

#[test]
fn migration_generate_from_diff_add_table_with_fk() {
    test_generate_migration("diff_add_table_with_fk", Vec::new());
}

#[test]
fn migration_generate_from_diff_drop_table_with_fk() {
    test_generate_migration("diff_drop_table_with_fk", Vec::new());
}

#[test]
fn migration_generate_from_diff_drop_table_all_the_types() {
    test_generate_migration("diff_drop_table_all_the_types", Vec::new());
}

#[test]
fn migration_generate_from_diff_add_table_all_the_types() {
    test_generate_migration("diff_add_table_all_the_types", Vec::new());
}

#[test]
fn migration_generate_from_diff_add_table_composite_key() {
    test_generate_migration("diff_add_table_composite_key", Vec::new());
}

#[test]
fn migration_generate_from_diff_drop_table_composite_key() {
    test_generate_migration("diff_drop_table_composite_key", Vec::new());
}

#[test]
fn migration_generate_from_diff_only_tables() {
    test_generate_migration("diff_only_tables", vec!["-o", "table_a"]);
}

#[test]
fn migration_generate_from_diff_except_tables() {
    test_generate_migration("diff_except_tables", vec!["-e", "table_b", "table_c"]);
}

#[cfg(feature = "sqlite")]
const BACKEND: &str = "sqlite";
#[cfg(feature = "postgres")]
const BACKEND: &str = "postgres";
#[cfg(feature = "mysql")]
const BACKEND: &str = "mysql";

fn backend_file_path(test_name: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("generate_migrations")
        .join(test_name)
        .join(BACKEND)
        .join(file)
}

fn test_generate_migration(test_name: &str, args: Vec<&str>) {
    let p = project(test_name).build();
    run_generate_migration_test(test_name, args, p);

    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("generate_migrations")
        .join(test_name)
        .join("diesel.toml");

    if Path::new(&config_path).exists() {
        let p = project(test_name)
            .file("diesel.toml", &read_file(&config_path))
            .build();

        run_generate_migration_test(test_name, Vec::new(), p);
    }
}

fn run_generate_migration_test(test_name: &str, args: Vec<&str>, p: Project) {
    let db = crate::support::database(&p.database_url());

    p.command("setup").run();

    let schema = read_file(&backend_file_path(test_name, "initial_schema.sql"));
    let schema = schema.trim();
    if !schema.is_empty() {
        db.execute(schema);
    }

    let mut schema_rs = backend_file_path(test_name, "schema.rs");
    if !schema_rs.exists() {
        schema_rs = backend_file_path(test_name, "../schema.rs");
    }

    let result = p.command("print-schema").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let initial_schema = result.stdout().replace("\r\n", "\n");

    let result = p
        .command("migration")
        .arg("generate")
        .arg(test_name)
        .arg("--version=12345")
        .arg(format!(
            "--diff-schema={schema_rs}",
            schema_rs = schema_rs.display()
        ))
        .args(args.clone())
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let up_sql = p.file_contents(format!("migrations/12345_{test_name}/up.sql"));
    let down_sql = p.file_contents(format!("migrations/12345_{test_name}/down.sql"));

    let mut setting = insta::Settings::new();
    setting.set_snapshot_path(backend_file_path(test_name, "up.sql"));
    setting.set_omit_expression(true);
    setting.set_description(format!("Test: {test_name}"));
    setting.set_prepend_module_to_snapshot(false);

    setting.bind(|| {
        insta::assert_snapshot!("expected", up_sql);
    });

    setting.set_snapshot_path(backend_file_path(test_name, "down.sql"));
    setting.bind(|| {
        insta::assert_snapshot!("expected", down_sql);
    });

    // check that "up.sql" works
    let result = p.command("migration").arg("run").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    // check that we can revert the migration
    let result = p.command("migration").arg("redo").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    // check that we get back the expected schema
    let result = p.command("print-schema").args(args).run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let result = result.stdout().replace("\r\n", "\n");

    let mut setting = insta::Settings::new();
    setting.set_snapshot_path(backend_file_path(test_name, "schema_out.rs"));
    setting.set_omit_expression(true);
    setting.set_description(format!("Test: {test_name}"));
    setting.set_prepend_module_to_snapshot(false);

    setting.bind(|| {
        insta::assert_snapshot!("expected", result);
    });

    // revert the migration and compare the schema to the initial one
    let result = p.command("migration").arg("revert").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let result = p.command("print-schema").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let final_schema = result.stdout().replace("\r\n", "\n");
    assert_eq!(final_schema, initial_schema);
}

fn read_file(path: &Path) -> String {
    let mut file = File::open(path).unwrap_or_else(|_| panic!("Could not open {}", path.display()));
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();
    string
}
