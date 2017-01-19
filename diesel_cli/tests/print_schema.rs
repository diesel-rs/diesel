use support::{database, project};

#[test]
fn run_infer_schema() {
    let p = project("print-schema").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE users1 (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE users2 (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("users1"));
    assert!(db.table_exists("users2"));

    let result = p.command("print-schema").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert_eq!(result.stdout(),
r"mod infer_users1 {
    table! {
        users1(id) {
            id -> Nullable<Integer>,
        }
    }
}
pub use self::infer_users1::*;
mod infer_users2 {
    table! {
        users2(id) {
            id -> Nullable<Integer>,
        }
    }
}
pub use self::infer_users2::*;

");
}

#[test]
fn run_infer_schema_whitelist() {
    let p = project("print-schema").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE users1 (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE users2 (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("users1"));
    assert!(db.table_exists("users2"));

    let result = p.command("print-schema").arg("users1").arg("-w").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert_eq!(result.stdout(),
r"mod infer_users1 {
    table! {
        users1(id) {
            id -> Nullable<Integer>,
        }
    }
}
pub use self::infer_users1::*;

");
}

#[test]
fn run_infer_schema_blacklist() {
    let p = project("print-schema").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE users1 (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE users2 (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("users1"));
    assert!(db.table_exists("users2"));

    let result = p.command("print-schema").arg("users1").arg("-b").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert_eq!(result.stdout(),
r"mod infer_users2 {
    table! {
        users2(id) {
            id -> Nullable<Integer>,
        }
    }
}
pub use self::infer_users2::*;

");
}
