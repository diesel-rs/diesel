use support::{database, project};

#[test]
fn run_infer_schema() {
    let p = project("print_schema").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE users1 (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE users2 (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("users1"));
    assert!(db.table_exists("users2"));

    let result = p.command("print-schema").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    if cfg!(feature = "sqlite") {
        assert_eq!(result.stdout(),
r"table! {
    users1 (id) {
        id -> Nullable<Integer>,
    }
}

table! {
    users2 (id) {
        id -> Nullable<Integer>,
    }
}
");
    } else if cfg!(feature = "postgres") {
                assert_eq!(result.stdout(),
r"table! {
    users1 (id) {
        id -> Int4,
    }
}

table! {
    users2 (id) {
        id -> Int4,
    }
}
");
    }
}

#[test]
fn run_infer_schema_whitelist() {
    let p = project("print_schema_whitelist").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE users1 (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE users2 (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("users1"));
    assert!(db.table_exists("users2"));

    let result = p.command("print-schema").arg("users1").arg("-w").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    if cfg!(feature = "sqlite") {
        assert_eq!(result.stdout(),
r"table! {
    users1 (id) {
        id -> Nullable<Integer>,
    }
}
");
    } else if cfg!(feature = "postgres") {
        assert_eq!(result.stdout(),
r"table! {
    users1 (id) {
        id -> Int4,
    }
}
");
    }
}

#[test]
fn run_infer_schema_blacklist() {
    let p = project("print_schema_blacklist").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE users1 (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE users2 (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("users1"));
    assert!(db.table_exists("users2"));

    let result = p.command("print-schema").arg("users1").arg("-b").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    if cfg!(feature = "sqlite") {
        assert_eq!(result.stdout(),
r"table! {
    users2 (id) {
        id -> Nullable<Integer>,
    }
}
");
    } else if cfg!(feature = "postgres") {
        assert_eq!(result.stdout(),
r"table! {
    users2 (id) {
        id -> Int4,
    }
}
");
    }
}

#[test]
fn run_infer_schema_order() {
    let p = project("print_schema_order").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE def (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE abc (id INTEGER PRIMARY KEY);");
    db.execute("CREATE TABLE ghi (id INTEGER PRIMARY KEY);");

    assert!(db.table_exists("def"));
    assert!(db.table_exists("abc"));
    assert!(db.table_exists("ghi"));

    let result = p.command("print-schema").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    if cfg!(feature = "sqlite") {
        assert_eq!(result.stdout(),
r"table! {
    abc (id) {
        id -> Nullable<Integer>,
    }
}

table! {
    def (id) {
        id -> Nullable<Integer>,
    }
}

table! {
    ghi (id) {
        id -> Nullable<Integer>,
    }
}
");
    } else if cfg!(feature = "postgres") {
        assert_eq!(result.stdout(),
r"table! {
    abc (id) {
        id -> Int4,
    }
}

table! {
    def (id) {
        id -> Int4,
    }
}

table! {
    ghi (id) {
        id -> Int4,
    }
}
");
    }
}

#[test]
fn run_infer_schema_compound_primary_key() {
    let p = project("print_schema_compound_primary_key").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    db.execute("CREATE TABLE asd (id INTEGER, qsd INTEGER, PRIMARY KEY (id, qsd));");

    let result = p.command("print-schema").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);

    if cfg!(feature = "sqlite") {
        assert_eq!(result.stdout(),
r"table! {
    asd (id, qsd) {
        id -> Nullable<Integer>,
        qsd -> Nullable<Integer>,
    }
}
");
    } else if cfg!(feature = "postgtres") {
        assert_eq!(result.stdout(),
r"table! {
    asd (id, qsd) {
        id -> Int4,
        qsd -> Int4,
    }
}
");
    }
}
