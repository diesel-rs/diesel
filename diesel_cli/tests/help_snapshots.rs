use crate::support::project;

#[test]
fn main_help() {
    let res = project("main-help").build().command("help").run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn setup_help() {
    let res = project("setup-help")
        .build()
        .command("setup")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn database_help() {
    let res = project("database-help")
        .build()
        .command("database")
        .arg("help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn database_reset_help() {
    let res = project("database-reset-help")
        .build()
        .command("database")
        .arg("reset")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn database_setup_help() {
    let res = project("database-setup-help")
        .build()
        .command("database")
        .arg("setup")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn completions_help() {
    let res = project("completions-help")
        .build()
        .command("completions")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn print_schema_help() {
    let res = project("print-schema-help")
        .build()
        .command("print-schema")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_help() {
    let res = project("migration-help")
        .build()
        .command("migration")
        .arg("help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_run_help() {
    let res = project("migration-run-help")
        .build()
        .command("migration")
        .arg("run")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_revert_help() {
    let res = project("migration-revert-help")
        .build()
        .command("migration")
        .arg("revert")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_redo_help() {
    let res = project("migration-redo-help")
        .build()
        .command("migration")
        .arg("redo")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_list_help() {
    let res = project("migration-list-help")
        .build()
        .command("migration")
        .arg("list")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_pending_help() {
    let res = project("migration-pending-help")
        .build()
        .command("migration")
        .arg("pending")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn migration_generate_help() {
    let res = project("migration-generate-help")
        .build()
        .command("migration")
        .arg("generate")
        .arg("--help")
        .run();
    assert!(res.is_success());
    insta::assert_snapshot!(res.stdout());
}

#[test]
fn version() {
    let res = project("version-output").build().command("--version").run();
    assert!(res.is_success());
    if cfg!(feature = "sqlite") {
        insta::assert_snapshot!("sqlite", res.stdout());
    } else if cfg!(feature = "postgres") {
        insta::assert_snapshot!("postgres", res.stdout());
    } else if cfg!(feature = "mysql") {
        insta::assert_snapshot!("mysql", res.stdout());
    } else {
        unreachable!();
    }
}
