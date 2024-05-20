#![allow(clippy::expect_fun_call)]
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::support::{database, project};

#[test]
fn run_infer_schema_without_docs() {
    test_print_schema("print_schema_simple_without_docs", vec![]);
}

#[test]
#[cfg(feature = "postgres")]
fn run_except_custom_type_definitions() {
    test_print_schema(
        "print_schema_except_custom_type_definitions",
        vec!["--except-custom-type-definitions", "MyType2"],
    );
}

#[test]
fn run_infer_schema() {
    test_print_schema("print_schema_simple", vec!["--with-docs"]);
}

#[test]
fn run_infer_schema_include() {
    test_print_schema(
        "print_schema_only_tables",
        vec!["--with-docs", "-o", "users1"],
    );
}

#[test]
fn run_infer_schema_include_regex() {
    test_print_schema(
        "print_schema_only_table_regexes",
        vec!["--with-docs", "-o", "users1"],
    );
}

#[test]
#[cfg(feature = "sqlite")]
fn run_infer_schema_django_bool_case() {
    test_print_schema(
        "print_schema_django_bool",
        vec!["--with-docs", "-o", "users1"],
    );
}

#[test]
fn run_infer_schema_exclude() {
    test_print_schema(
        "print_schema_except_tables",
        vec!["--with-docs", "-e", "users1"],
    );
}

#[test]
fn run_infer_schema_exclude_regex() {
    test_print_schema(
        "print_schema_except_table_regexes",
        vec!["--with-docs", "-e", "users1"],
    );
}

#[test]
fn run_infer_schema_table_order() {
    test_print_schema("print_schema_table_order", vec!["--with-docs"]);
}

#[test]
fn run_infer_schema_column_order() {
    test_print_schema(
        "print_schema_column_order",
        vec!["--column-sorting", "name"],
    );
}

#[test]
fn run_infer_schema_compound_primary_key() {
    test_print_schema("print_schema_compound_primary_key", vec!["--with-docs"]);
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_specifying_schema_name() {
    test_print_schema(
        "print_schema_specifying_schema_name",
        vec!["--with-docs", "--schema", "custom_schema"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_specifying_schema_name_with_foreign_keys() {
    test_print_schema(
        "print_schema_specifying_schema_name_with_foreign_keys",
        vec!["--with-docs", "--schema", "custom_schema"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_with_compound_foreign_keys() {
    test_print_schema(
        "print_schema_with_compound_foreign_keys",
        vec!["--with-docs"],
    );
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_with_foreign_keys() {
    test_print_schema("print_schema_with_foreign_keys", vec!["--with-docs"]);
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_with_foreign_keys_reserved_names() {
    test_print_schema(
        "print_schema_with_foreign_keys_reserved_names",
        vec!["--with-docs"],
    );
}

#[test]
fn print_schema_column_renaming() {
    test_print_schema("print_schema_column_renaming", vec!["--with-docs"]);
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_type_renaming() {
    test_print_schema("print_schema_type_renaming", vec!["--with-docs"]);
}

#[test]
#[cfg(feature = "mysql")]
fn print_schema_unsigned() {
    test_print_schema("print_schema_unsigned", vec!["--with-docs"]);
}

#[test]
#[cfg(feature = "mysql")]
fn print_schema_datetime_for_mysql() {
    test_print_schema("print_schema_datetime_for_mysql", vec!["--with-docs"]);
}

#[test]
#[cfg(not(windows))]
fn print_schema_patch_file() {
    let path_to_patch_file = backend_file_path("print_schema_patch_file", "schema.patch");
    let path = path_to_patch_file.display().to_string();
    test_print_schema("print_schema_patch_file", vec!["--patch-file", &path]);
}

#[test]
fn print_schema_custom_types() {
    test_print_schema(
        "print_schema_custom_types",
        vec!["--import-types", "foo::*", "--import-types", "bar::*"],
    );
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_custom_types_custom_schema() {
    test_print_schema(
        "print_schema_custom_types_custom_schema",
        vec![
            "--schema",
            "v2",
            "--custom-type-derives",
            "diesel::query_builder::QueryId",
            "--custom-type-derives",
            "Clone",
        ],
    );
}

#[test]
fn print_schema_with_unmappable_names() {
    test_print_schema("print_schema_with_unmappable_names", vec!["--with-docs"]);
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_with_unmappable_names_and_schema_name() {
    test_print_schema(
        "print_schema_with_unmappable_names_and_schema_name",
        vec!["--with-docs", "--schema", "custom_schema"],
    )
}

#[test]
fn print_schema_with_separate_unique_constraint_and_foreign_key() {
    test_print_schema("print_schema_regression_test_for_2623", vec![])
}

#[test]
fn schema_file_is_relative_to_project_root() {
    let p = project("schema_file_is_relative_to_project_root")
        .folder("foo")
        .build();
    let _db = database(&p.database_url());

    p.command("setup").run();
    p.command("migration").arg("run").cd("foo").run();

    assert!(p.has_file("src/schema.rs"));
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_disabling_custom_type_works() {
    test_print_schema(
        "print_schema_disabling_custom_type_works",
        vec!["--no-generate-missing-sql-type-definitions"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_default_is_to_generate_custom_types() {
    test_print_schema(
        "print_schema_default_is_to_generate_custom_types",
        vec!["--with-docs"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_specifying_schema_name_with_custom_type() {
    test_print_schema(
        "print_schema_specifying_schema_name_with_custom_type",
        vec!["--with-docs", "--schema", "custom_schema"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_custom_types_check_default_derives() {
    test_print_schema(
        "print_schema_custom_types_check_default_derives",
        vec!["--with-docs"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_custom_types_overriding_derives_works() {
    test_print_schema(
        "print_schema_custom_types_overriding_derives_works",
        vec![
            "--with-docs",
            "--custom-type-derives",
            "diesel::sql_types::SqlType",
            "--custom-type-derives",
            "core::fmt::Debug",
        ],
    )
}

#[test]
#[cfg(feature = "sqlite")]
fn print_schema_generated_columns() {
    test_print_schema("print_schema_generated_columns", vec![])
}

#[test]
#[cfg(feature = "sqlite")]
fn print_schema_generated_columns_with_generated_always() {
    test_print_schema("print_schema_generated_columns_generated_always", vec![])
}

#[test]
#[cfg(feature = "sqlite")]
fn print_schema_sqlite_rowid_column() {
    test_print_schema(
        "print_schema_sqlite_rowid_column",
        vec!["--sqlite-integer-primary-key-is-bigint"],
    )
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_multiple_annotations() {
    test_print_schema("print_schema_multiple_annotations", vec![])
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_array_type() {
    test_print_schema("print_schema_array_type", vec![])
}

#[test]
#[cfg(feature = "sqlite")]
fn print_schema_sqlite_implicit_foreign_key_reference() {
    test_print_schema("print_schema_sqlite_implicit_foreign_key_reference", vec![]);
}

#[test]
#[cfg(feature = "sqlite")]
fn print_schema_sqlite_without_explicit_primary_key() {
    test_print_schema("print_schema_sqlite_without_explicit_primary_key", vec![])
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_respects_type_name_case() {
    test_print_schema("print_schema_respects_type_name_case", vec!["--with-docs"])
}

#[test]
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn print_schema_comments_fallback_on_generated() {
    test_print_schema(
        "print_schema_comments_fallback_on_generated",
        vec!["--with-docs"],
    )
}

#[test]
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn print_schema_with_enum_set_types() {
    test_print_schema(
        "print_schema_with_enum_set_types",
        vec![
            "--with-docs",
            "--custom-type-derives",
            "diesel::query_builder::QueryId",
            "--custom-type-derives",
            "Clone",
        ],
    )
}

#[test]
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn print_schema_comments_dont_fallback_on_generated() {
    test_print_schema(
        "print_schema_comments_dont_fallback_on_generated",
        vec!["--with-docs-config", "only-database-comments"],
    )
}

#[test]
fn print_schema_reserved_names() {
    test_print_schema("print_schema_reserved_name_mitigation_issue_3404", vec![])
}

#[test]
#[cfg(feature = "postgres")]
fn print_schema_regression_3446_ignore_compound_foreign_keys() {
    test_print_schema("print_schema_regression_3446_compound_keys", vec![])
}

#[test]
fn print_schema_several_keys_with_compound_key() {
    test_print_schema("print_schema_several_keys_with_compound_key", vec![])
}

// some mysql versions concert quoted table names to lowercase
// anyway
#[cfg(any(feature = "postgres", feature = "sqlite"))]
#[test]
fn print_schema_quoted_table_name() {
    test_print_schema("print_schema_quoted_table_name", vec![])
}

#[cfg(feature = "postgres")]
#[test]
fn print_schema_quoted_schema_and_table_name() {
    test_print_schema(
        "print_schema_quoted_schema_and_table_name",
        vec!["--schema", "CustomSchema"],
    )
}

#[cfg(feature = "postgres")]
#[test]
fn print_schema_citext() {
    test_print_schema("print_schema_citext", vec![])
}

#[test]
fn print_schema_with_multiple_schema() {
    test_multiple_print_schema(
        "print_schema_with_multiple_schema",
        vec![
            "--schema-key",
            "default",
            "--schema-key",
            "user1",
            "-o",
            "users1",
            "--with-docs",
            "--schema-key",
            "user2",
            "-o",
            "users2",
            "--with-docs",
        ],
    )
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
        .join("print_schema")
        .join(test_name)
        .join(BACKEND)
        .join(file)
}

fn test_multiple_print_schema(test_name: &str, args: Vec<&str>) {
    let test_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("print_schema")
        .join(test_name);
    let p = project(test_name)
        .file(
            "diesel.toml",
            r#"
            [print_schema.user1]
            [print_schema.user2]
            "#,
        )
        .build();
    let db = database(&p.database_url());

    p.command("setup").run();

    let schema = read_file(&backend_file_path(test_name, "schema.sql"));
    db.execute(&schema);

    let result = p.command("print-schema").args(args).run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let result = result.stdout().replace("\r\n", "\n");

    let mut setting = insta::Settings::new();
    setting.set_snapshot_path(backend_file_path(test_name, ""));
    setting.set_omit_expression(true);
    setting.set_description(format!("Test: {test_name}"));
    setting.set_prepend_module_to_snapshot(false);

    setting.bind(|| {
        insta::assert_snapshot!("expected", result);
        test_multiple_print_schema_config(test_name, &test_path, schema);
    });
}

fn test_print_schema(test_name: &str, args: Vec<&str>) {
    let test_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("print_schema")
        .join(test_name);
    let p = project(test_name).build();
    let db = database(&p.database_url());

    p.command("setup").run();

    let schema = read_file(&backend_file_path(test_name, "schema.sql"));
    db.execute(&schema);

    let result = p.command("print-schema").args(args).run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let result = result.stdout().replace("\r\n", "\n");

    let mut setting = insta::Settings::new();
    setting.set_snapshot_path(backend_file_path(test_name, ""));
    setting.set_omit_expression(true);
    setting.set_description(format!("Test: {test_name}"));
    setting.set_prepend_module_to_snapshot(false);

    setting.bind(|| {
        insta::assert_snapshot!("expected", result);

        test_print_schema_config(test_name, &test_path, schema);
    });
}

fn test_print_schema_config(test_name: &str, test_path: &Path, schema: String) {
    let config = read_file(&test_path.join("diesel.toml"));
    let mut p = project(&format!("{}_config", test_name)).file("diesel.toml", &config);

    let patch_file = backend_file_path(test_name, "schema.patch");
    if patch_file.exists() {
        let patch_contents = read_file(&patch_file);
        p = p.file("schema.patch", &patch_contents);
    }

    let p = p.build();

    p.command("setup").run();
    p.create_migration("12345_create_schema", &schema, None, None);
    let result = p.command("migration").arg("run").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let schema = p.file_contents("src/schema.rs").replace("\r\n", "\n");
    insta::assert_snapshot!("expected", schema);

    let result = p.command("print-schema").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let result = result.stdout().replace("\r\n", "\n");

    insta::assert_snapshot!("expected", result);
}

fn test_multiple_print_schema_config(test_name: &str, test_path: &Path, schema: String) {
    let config = read_file(&test_path.join("diesel.toml"));
    let mut p = project(&format!("{}_config", test_name)).file("diesel.toml", &config);

    let patch_file = backend_file_path(test_name, "schema.patch");
    if patch_file.exists() {
        let patch_contents = read_file(&patch_file);
        p = p.file("schema.patch", &patch_contents);
    }

    let p = p.build();

    p.command("setup").run();
    p.create_migration("12345_create_schema", &schema, None, None);
    let result = p.command("migration").arg("run").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let schema = p.file_contents("src/schema1.rs").replace("\r\n", "\n");
    insta::assert_snapshot!("expected_1", schema);
    let schema = p.file_contents("src/schema2.rs").replace("\r\n", "\n");
    insta::assert_snapshot!("expected_2", schema);

    let result = p.command("print-schema").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let result = result.stdout().replace("\r\n", "\n");
    insta::assert_snapshot!("expected", result);
}

fn read_file(path: &Path) -> String {
    let mut file = File::open(path).expect(&format!("Could not open {}", path.display()));
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();
    string
}
