use crate::schema::*;
use diesel::result::Error;
use diesel::*;

#[test]
#[cfg(not(feature = "sqlite"))] // FIXME: This test is only valid when operating on a file and not :memory:
fn transaction_executes_fn_in_a_sql_transaction() {
    const TEST_NAME: &str = "transaction_executes_fn_in_a_sql_transaction";
    let conn1 = &mut connection_without_transaction();
    let conn2 = &mut connection_without_transaction();
    setup_test_table(conn1, TEST_NAME);

    fn get_count(conn: &mut TestConnection) -> i64 {
        count_test_table(conn, TEST_NAME)
    }

    conn1
        .transaction::<_, Error, _>(|conn1| {
            assert_eq!(0, get_count(conn1));
            assert_eq!(0, get_count(conn2));
            diesel::sql_query(format!("INSERT INTO {TEST_NAME} DEFAULT VALUES")).execute(conn1)?;
            assert_eq!(1, get_count(conn1));
            assert_eq!(0, get_count(conn2));
            Ok(())
        })
        .unwrap();

    assert_eq!(1, get_count(conn1));
    assert_eq!(1, get_count(conn2));

    drop_test_table(conn1, TEST_NAME);
}

#[test]
fn transaction_returns_the_returned_value() {
    let conn1 = &mut connection_without_transaction();

    assert_eq!(Ok(1), conn1.transaction::<_, Error, _>(|_| Ok(1)));
}

#[test]
fn transaction_is_rolled_back_when_returned_an_error() {
    let connection = &mut connection_without_transaction();
    let test_name = "transaction_is_rolled_back_when_returned_an_error";
    setup_test_table(connection, test_name);

    let _ = connection.transaction::<(), _, _>(|connection| {
        diesel::sql_query(format!("INSERT INTO {test_name} DEFAULT VALUES"))
            .execute(connection)
            .unwrap();
        Err(Error::RollbackTransaction)
    });
    assert_eq!(0, count_test_table(connection, test_name));

    drop_test_table(connection, test_name);
}

// This test uses a SQLite3 fact to generate a rollback error,
// so that we can verify error. Reference:
// https://www.sqlite.org/lang_transaction.html
//
// The same trick cannot be used for PostgreSQL as it generates
// warning, but not error if a rollback is called twice. Reference:
// https://www.postgresql.org/docs/9.4/sql-rollback.html
//
// The same trick seems to work for MySQL as well based on the
// test result, but I cannot find a document support yet. Hence
// this test is marked for "sqlite" only as this moment. FIXME.
#[test]
#[cfg(feature = "sqlite")]
fn transaction_rollback_returns_error() {
    let connection = &mut connection_without_transaction();
    let test_name = "transaction_rollback_returns_error";
    setup_test_table(connection, test_name);

    // Create a transaction that will fail to rollback.
    let r = connection.transaction::<usize, _, _>(|connection| {
        diesel::sql_query(format!("INSERT INTO {test_name} DEFAULT VALUES"))
            .execute(connection)
            .unwrap();

        // This rollback would succeed, and cause any rollback later to fail.
        diesel::sql_query("ROLLBACK").execute(connection).unwrap();

        // Return any error to trigger a rollback that fails in this case.
        Err(Error::NotFound)
    });

    // Verify that the transaction failed with an error from database (and not the original
    // "NotFound").
    assert!(matches!(r.unwrap_err(), Error::DatabaseError(_, _)));

    assert_eq!(0, count_test_table(connection, test_name));
    drop_test_table(connection, test_name);
}

#[test]
fn transactions_can_be_nested() {
    let connection = &mut connection_without_transaction();
    const TEST_NAME: &str = "transactions_can_be_nested";
    setup_test_table(connection, TEST_NAME);
    fn get_count(connection: &mut TestConnection) -> i64 {
        count_test_table(connection, TEST_NAME)
    }

    let _ = connection.transaction::<(), _, _>(|connection| {
        diesel::sql_query(format!("INSERT INTO {TEST_NAME} DEFAULT VALUES"))
            .execute(connection)
            .unwrap();
        assert_eq!(1, get_count(connection));
        let _ = connection.transaction::<(), _, _>(|connection| {
            diesel::sql_query(format!("INSERT INTO {TEST_NAME} DEFAULT VALUES"))
                .execute(connection)
                .unwrap();
            assert_eq!(2, get_count(connection));
            Err(Error::RollbackTransaction)
        });
        assert_eq!(1, get_count(connection));
        let _ = connection.transaction::<(), Error, _>(|connection| {
            diesel::sql_query(format!("INSERT INTO {TEST_NAME} DEFAULT VALUES"))
                .execute(connection)
                .unwrap();
            assert_eq!(2, get_count(connection));
            Ok(())
        });
        assert_eq!(2, get_count(connection));
        Err(Error::RollbackTransaction)
    });
    assert_eq!(0, get_count(connection));

    drop_test_table(connection, TEST_NAME);
}

#[test]
fn test_transaction_always_rolls_back() {
    let connection = &mut connection_without_transaction();
    let test_name = "test_transaction_always_rolls_back";
    setup_test_table(connection, test_name);

    let result = connection.test_transaction::<_, Error, _>(|connection| {
        diesel::sql_query(format!("INSERT INTO {test_name} DEFAULT VALUES")).execute(connection)?;
        assert_eq!(1, count_test_table(connection, test_name));
        Ok("success")
    });
    assert_eq!(0, count_test_table(connection, test_name));
    assert_eq!("success", result);

    drop_test_table(connection, test_name);
}

#[test]
#[should_panic(expected = "Transaction did not succeed")]
fn test_transaction_panics_on_error() {
    let connection = &mut connection_without_transaction();
    connection.test_transaction::<(), _, _>(|_| Err(()));
}

fn setup_test_table(connection: &mut TestConnection, table_name: &str) {
    use crate::schema_dsl::*;
    create_table(table_name, (integer("id").primary_key().auto_increment(),))
        .execute(connection)
        .unwrap();
}

fn drop_test_table(connection: &mut TestConnection, table_name: &str) {
    diesel::sql_query(format!("DROP TABLE {table_name}"))
        .execute(connection)
        .unwrap();
}

fn count_test_table(connection: &mut TestConnection, table_name: &str) -> i64 {
    use diesel::dsl::sql;
    select(sql::<sql_types::BigInt>(&format!(
        "COUNT(*) FROM {table_name}"
    )))
    .first(connection)
    .unwrap()
}

#[test]
#[cfg(feature = "postgres")]
fn regression_test_for_2123() {
    let conn = &mut connection_without_transaction();
    // fail once
    let ret = conn.transaction(|conn| {
        let _ = conn.transaction(|conn| {
            // handling error
            match diesel::sql_query("SELECT foo").execute(conn) {
                // do nothing
                Ok(_) => unreachable!("This query should fail"),
                // ignore the error
                Err(e) => eprintln!("error occurred: {e}"),
            };
            Ok::<_, Error>(())
        });

        conn.transaction(|conn| {
            let ret = diesel::sql_query("SELECT 1").execute(conn);
            assert_eq!(Ok(1), ret);
            Ok::<_, Error>(())
        })
    });
    println!("{ret:?}");
    // other transaction
    let ret = conn
        .build_transaction()
        .serializable()
        .run(|conn| diesel::sql_query("SELECT 1").execute(conn));
    // must be Ok(1), but get Err(AlreadyInTransaction)
    println!("{ret:?}");
    assert_eq!(Ok(1), ret);
}
