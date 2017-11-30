use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use support::{database, project};

#[test]
fn run_infer_schema_without_docs() {
    test_print_schema("print_schema_simple_without_docs", vec![]);
}

#[test]
fn run_infer_schema() {
    test_print_schema("print_schema_simple", vec!["--with-docs"]);
}

#[test]
fn run_infer_schema_whitelist() {
    test_print_schema(
        "print_schema_whitelist",
        vec!["--with-docs", "-w", "users1"],
    );
}

#[test]
fn run_infer_schema_blacklist() {
    test_print_schema(
        "print_schema_blacklist",
        vec!["--with-docs", "-b", "users1"],
    );
}

#[test]
fn run_infer_schema_order() {
    test_print_schema("print_schema_order", vec!["--with-docs"]);
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
fn print_schema_with_foreign_keys() {
    test_print_schema("print_schema_with_foreign_keys", vec!["--with-docs"]);
}

#[test]
fn print_schema_column_renaming() {
    test_print_schema("print_schema_column_renaming", vec!["--with-docs"]);
}

#[cfg(feature = "sqlite")]
const BACKEND: &str = "sqlite";
#[cfg(feature = "postgres")]
const BACKEND: &str = "postgres";
#[cfg(feature = "mysql")]
const BACKEND: &str = "mysql";

fn test_print_schema(test_name: &str, args: Vec<&str>) {
    let test_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("print_schema")
        .join(test_name)
        .join(BACKEND);
    let p = project(test_name).build();
    let db = database(&p.database_url());

    p.command("setup").run();

    let schema = read_file(&test_path.join("schema.sql"));
    db.execute(&schema);

    let result = p.command("print-schema").args(args).run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let expected = read_file(&test_path.join("expected.rs"));

    assert_diff!(&expected, result.stdout(), "\n", 0);
}

fn read_file(path: &Path) -> String {
    println!("{}", path.display());
    let mut file = File::open(path).expect(&format!("Could not open {}", path.display()));
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();
    string
}
