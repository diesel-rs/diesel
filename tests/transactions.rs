use schema::*;
use yaqb::*;

macro_rules! try_no_coerce {
    ($e:expr) => ({
        match $e {
            Ok(e) => e,
            Err(e) => return Err(e),
        }
    })
}

#[test]
fn transaction_executes_fn_in_a_sql_transaction() {
    let conn1 = connection_without_transaction();
    let conn2 = connection_without_transaction();
    let test_name = "transaction_executes_fn_in_a_sql_transaction";
    setup_test_table(&conn1, test_name);
    let get_count = |conn| count_test_table(conn, test_name);

    conn1.transaction(|| {
        assert_eq!(0, get_count(&conn1));
        assert_eq!(0, get_count(&conn2));
        try_no_coerce!(conn1.execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name)));
        assert_eq!(1, get_count(&conn1));
        assert_eq!(0, get_count(&conn2));
        Ok(())
    }).unwrap();

    assert_eq!(1, get_count(&conn1));
    assert_eq!(1, get_count(&conn2));

    drop_test_table(&conn1, test_name);
}

#[test]
fn transaction_returns_the_returned_value() {
    let conn1 = connection_without_transaction();

    assert_eq!(Ok(1), conn1.transaction::<_, (), _>(|| Ok(1)));
}

#[test]
fn transaction_is_rolled_back_when_returned_an_error() {
    let connection = connection_without_transaction();
    let test_name = "transaction_is_rolled_back_when_returned_an_error";
    setup_test_table(&connection, test_name);
    let get_count = || count_test_table(&connection, test_name);

    let _ = connection.transaction::<(), (), _>(|| {
        connection.execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name)).unwrap();
        Err(())
    });
    assert_eq!(0, get_count());

    drop_test_table(&connection, test_name);
}

fn setup_test_table(connection: &Connection, table_name: &str) {
    connection.execute(&format!("CREATE TABLE {} (id SERIAL PRIMARY KEY)", table_name)).unwrap();
}

fn drop_test_table(connection: &Connection, table_name: &str) {
    connection.execute(&format!("DROP TABLE IF EXISTS {}", table_name)).unwrap();
}

fn count_test_table(connection: &Connection, table_name: &str) -> i64 {
    let query = format!("SELECT COUNT(*) FROM {}", table_name);
    let mut cursor = connection.query_sql::<types::BigInt, _>(&query).unwrap();
    cursor.nth(0).unwrap()
}
