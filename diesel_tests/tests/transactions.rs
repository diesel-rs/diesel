use diesel::result::Error;
use diesel::*;
use schema::*;

#[test]
#[cfg(not(feature = "sqlite"))] // FIXME: This test is only valid when operating on a file and not :memory:
fn transaction_executes_fn_in_a_sql_transaction() {
    let conn1 = connection_without_transaction();
    let conn2 = connection_without_transaction();
    let test_name = "transaction_executes_fn_in_a_sql_transaction";
    setup_test_table(&conn1, test_name);
    let get_count = |conn| count_test_table(conn, test_name);

    conn1
        .transaction::<_, Error, _>(|| {
            assert_eq!(0, get_count(&conn1));
            assert_eq!(0, get_count(&conn2));
            conn1.execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name))?;
            assert_eq!(1, get_count(&conn1));
            assert_eq!(0, get_count(&conn2));
            Ok(())
        })
        .unwrap();

    assert_eq!(1, get_count(&conn1));
    assert_eq!(1, get_count(&conn2));

    drop_test_table(&conn1, test_name);
}

#[test]
fn transaction_returns_the_returned_value() {
    let conn1 = connection_without_transaction();

    assert_eq!(Ok(1), conn1.transaction::<_, Error, _>(|| Ok(1)));
}

#[test]
fn transaction_is_rolled_back_when_returned_an_error() {
    let connection = connection_without_transaction();
    let test_name = "transaction_is_rolled_back_when_returned_an_error";
    setup_test_table(&connection, test_name);
    let get_count = || count_test_table(&connection, test_name);

    let _ = connection.transaction::<(), _, _>(|| {
        connection
            .execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name))
            .unwrap();
        Err(Error::RollbackTransaction)
    });
    assert_eq!(0, get_count());

    drop_test_table(&connection, test_name);
}

#[test]
fn transactions_can_be_nested() {
    let connection = connection_without_transaction();
    let test_name = "transactions_can_be_nested";
    setup_test_table(&connection, test_name);
    let get_count = || count_test_table(&connection, test_name);

    let _ = connection.transaction::<(), _, _>(|| {
        connection
            .execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name))
            .unwrap();
        assert_eq!(1, get_count());
        let _ = connection.transaction::<(), _, _>(|| {
            connection
                .execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name))
                .unwrap();
            assert_eq!(2, get_count());
            Err(Error::RollbackTransaction)
        });
        assert_eq!(1, get_count());
        let _ = connection.transaction::<(), Error, _>(|| {
            connection
                .execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name))
                .unwrap();
            assert_eq!(2, get_count());
            Ok(())
        });
        assert_eq!(2, get_count());
        Err(Error::RollbackTransaction)
    });
    assert_eq!(0, get_count());

    drop_test_table(&connection, test_name);
}

#[test]
fn test_transaction_always_rolls_back() {
    let connection = connection_without_transaction();
    let test_name = "test_transaction_always_rolls_back";
    setup_test_table(&connection, test_name);

    let result = connection.test_transaction::<_, Error, _>(|| {
        connection.execute(&format!("INSERT INTO {} DEFAULT VALUES", test_name))?;
        assert_eq!(1, count_test_table(&connection, test_name));
        Ok("success")
    });
    assert_eq!(0, count_test_table(&connection, test_name));
    assert_eq!("success", result);

    drop_test_table(&connection, test_name);
}

#[test]
#[should_panic(expected = "Transaction did not succeed")]
fn test_transaction_panics_on_error() {
    let connection = connection_without_transaction();
    connection.test_transaction::<(), _, _>(|| Err(()));
}

fn setup_test_table(connection: &TestConnection, table_name: &str) {
    use schema_dsl::*;
    create_table(table_name, (integer("id").primary_key().auto_increment(),))
        .execute(connection)
        .unwrap();
}

fn drop_test_table(connection: &TestConnection, table_name: &str) {
    connection
        .execute(&format!("DROP TABLE {}", table_name))
        .unwrap();
}

fn count_test_table(connection: &TestConnection, table_name: &str) -> i64 {
    use diesel::dsl::sql;
    select(sql::<sql_types::BigInt>(&format!(
        "COUNT(*) FROM {}",
        table_name
    )))
    .first(connection)
    .unwrap()
}
