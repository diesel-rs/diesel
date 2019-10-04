use diesel::{sql_query, Connection, RunQueryDsl, SqliteConnection};

fn conn() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
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
fn test_extension_function_ok() {
    let conn = conn();

    // enable loading
    conn.enable_load_extension().unwrap();

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

#[test]
fn test_extension_function_fail() {
    let conn = conn();

    // load mod_spatialite
    sql_query("SELECT load_extension('mod_spatialite.so');")
        .execute(&conn)
        .expect("Failed to load mod_spatialite.so");

    // test module function
    let r = 
        sql_query("SELECT * FROM (SELECT 0 as id, AsText(ST_Point(25.2,54.2)) as bar) foo;")
            .load(&conn);
           
    assert!(r.is_err());
}
