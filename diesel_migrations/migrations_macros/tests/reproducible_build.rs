//! Building the same crate from two directories must give identical
//! crate metadata, even when it uses `embed_migrations!`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Stubs just enough of `diesel_migrations` for the expansion to compile.
const FIXTURE_LIB_RS: &str = r#"
#![allow(non_snake_case)]

mod diesel_migrations {
    pub mod EmbeddedMigrations { pub fn new<T>(_: T) {} }
    pub mod EmbeddedMigration { pub fn new<A, B, C, D>(_: A, _: B, _: C, _: D) {} }
    pub mod EmbeddedName { pub fn new<T>(_: T) {} }
    pub mod TomlMetadataWrapper { pub fn new<T>(_: T) {} }
}

pub fn migrations() {
    migrations_macros::embed_migrations!()
}
"#;

#[test]
fn embed_migrations_is_reproducible_across_directories() {
    let work_dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    // A shared target dir keeps the dependencies identical between the
    // two builds, so only the fixture itself can differ.
    let target_dir = work_dir.path().join("target");
    let first = build_fixture(&work_dir.path().join("a"), &target_dir);
    let second = build_fixture(&work_dir.path().join("b"), &target_dir);
    assert!(
        first == second,
        "crate metadata differs between the two build directories"
    );
}

/// Builds the fixture crate in `root` and returns its rmeta.
fn build_fixture(root: &Path, target_dir: &Path) -> Vec<u8> {
    let migration = root.join("migrations/2025-01-01-000000_init");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(&migration).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
migrations_macros = {{ path = '{}' }}

[workspace]
"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), FIXTURE_LIB_RS).unwrap();
    std::fs::write(migration.join("up.sql"), "CREATE TABLE t (id INTEGER);").unwrap();
    std::fs::write(migration.join("down.sql"), "DROP TABLE t;").unwrap();

    // The macro canonicalizes the migrations path, which can spell the
    // directory differently, so we remap both spellings.
    let canonical_root = root.canonicalize().unwrap();
    let status = Command::new("cargo")
        .arg("rustc")
        .arg("--offline")
        .arg("--quiet")
        .arg(format!(
            "--manifest-path={}",
            root.join("Cargo.toml").display()
        ))
        .arg(format!("--target-dir={}", target_dir.display()))
        .arg("--")
        .arg(format!("--remap-path-prefix={}=/fixture", root.display()))
        .arg(format!(
            "--remap-path-prefix={}=/fixture",
            canonical_root.display()
        ))
        .env_remove("RUSTFLAGS")
        .status()
        .unwrap();
    assert!(status.success(), "failed to build the fixture crate");

    let mut rmetas = Vec::new();
    find_fixture_rmetas(target_dir, &mut rmetas);
    assert_eq!(rmetas.len(), 1, "expected one fixture rmeta: {rmetas:?}");
    std::fs::read(&rmetas[0]).unwrap()
}

fn find_fixture_rmetas(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if path.is_dir() {
            find_fixture_rmetas(&path, found);
        } else if name.starts_with("libfixture-") && name.ends_with(".rmeta") {
            found.push(path);
        }
    }
}
