use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::support::{database, project};

#[test]
fn run_infer_schema_without_docs() {
    test_print_schema("print_schema_simple_without_docs", vec![]);
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
fn print_schema_with_seperate_unique_constraint_and_foreign_key() {
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
    let expected = read_file(&backend_file_path(test_name, "expected.rs")).replace("\r\n", "\n");

    let result = result.stdout().replace("\r\n", "\n");

    assert_diff!(&expected, &result, "\n", 0);

    test_print_schema_config(test_name, &test_path, schema, expected);
}

fn test_print_schema_config(test_name: &str, test_path: &Path, schema: String, expected: String) {
    let config = read_file(&test_path.join("diesel.toml"));
    let mut p = project(&format!("{}_config", test_name)).file("diesel.toml", &config);

    let patch_file = backend_file_path(test_name, "schema.patch");
    if patch_file.exists() {
        let patch_contents = read_file(&patch_file);
        p = p.file("schema.patch", &patch_contents);
    }

    let p = p.build();

    p.command("setup").run();
    p.create_migration("12345_create_schema", &schema, "");

    let result = p.command("migration").arg("run").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let schema = p.file_contents("src/schema.rs").replace("\r\n", "\n");
    assert_diff!(&expected, &schema, "\n", 0);

    let result = p.command("print-schema").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    let result = result.stdout().replace("\r\n", "\n");
    assert_diff!(&expected, &result, "\n", 0);
}

fn read_file(path: &Path) -> String {
    let mut file = File::open(path).expect(&format!("Could not open {}", path.display()));
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();
    string
}
