use crate::support::{database, project};

#[test]
fn database_drop_drops_database() {
    let p = project("database_drop").build();
    let db = database(&p.database_url()).create();

    assert!(db.exists());

    let result = p.command("database").arg("drop").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Dropping database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(!db.exists());
}

#[test]
fn database_drop_does_not_print_to_stdout_if_no_db_exists() {
    let p = project("database_drop_no_stdout").build();
    let db = database(&p.database_url());

    assert!(!db.exists());

    let result = p.command("database").arg("drop").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        !result.stdout().contains("Dropping database:"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(!db.exists());
}
