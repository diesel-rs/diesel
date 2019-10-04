use diesel::{sql_query, Connection, RunQueryDsl, SqliteConnection};

fn conn() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

#[test]
fn test_load_extension_fail() {
    let conn = conn();

    // disable extension loading
    conn.enable_load_extension(false).unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('mod_spatialite.so');").execute(&conn);

    assert!(result.is_err());
}

#[test]
fn test_load_extension_ok() {
    let conn = conn();

    // enable loading
    conn.enable_load_extension(true).unwrap();

    // load mod_spatialite
    let result = sql_query("SELECT load_extension('mod_spatialite.so');").execute(&conn);

    assert!(result.is_ok());
}

table! {
    foo {
        id -> Integer,
        bar -> VarChar,
    }
}

#[derive(QueryableByName)]
#[table_name = "foo"]
struct Foo {
    id: i32,
    bar: String,
}

#[test]
fn test_extension_function() {
    let conn = conn();

    // enable loading
    conn.enable_load_extension(true).unwrap();

    // load mod_spatialite
    sql_query("SELECT load_extension('mod_spatialite.so');")
        .execute(&conn)
        .expect("Failed to load mod_spatialite.so");

    // test module function
    let r: Vec<Foo> =
        sql_query("SELECT * FROM (SELECT 0 as id, AsText(ST_Point(25.2,54.2)) as bar) foo;")
            .load(&conn)
            .expect("Failed to query lib version");

    for v in r {
        assert_eq!(&v.bar, "POINT(25.2 54.2)");
    }
}
