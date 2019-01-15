use diesel::result::DatabaseErrorKind::{ForeignKeyViolation, UniqueViolation};
use diesel::result::Error::DatabaseError;
use diesel::*;
use schema::*;

#[test]
fn unique_constraints_are_detected() {
    let connection = connection();
    insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .execute(&connection)
        .unwrap();

    let failure = insert_into(users::table)
        .values(&User::new(1, "Jim"))
        .execute(&connection);
    assert_matches!(failure, Err(DatabaseError(UniqueViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn unique_constraints_report_correct_constraint_name() {
    let connection = connection();
    connection
        .execute("CREATE UNIQUE INDEX users_name ON users (name)")
        .unwrap();
    insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .execute(&connection)
        .unwrap();

    let failure = insert_into(users::table)
        .values(&User::new(2, "Sean"))
        .execute(&connection);
    match failure {
        Err(DatabaseError(UniqueViolation, e)) => {
            assert_eq!(Some("users"), e.table_name());
            assert_eq!(None, e.column_name());
            assert_eq!(Some("users_name"), e.constraint_name());
        }
        _ => panic!(
            "{:?} did not match Err(DatabaseError(UniqueViolation, e))",
            failure
        ),
    };
}

macro_rules! try_no_coerce {
    ($e:expr) => {{
        match $e {
            Ok(e) => e,
            Err(e) => return Err(e),
        }
    }};
}

#[test]
fn cached_prepared_statements_can_be_reused_after_error() {
    let connection = connection_without_transaction();
    let user = User::new(1, "Sean");
    let query = insert_into(users::table).values(&user);

    connection.test_transaction(|| {
        try_no_coerce!(query.execute(&connection));

        let failure = query.execute(&connection);
        assert_matches!(failure, Err(DatabaseError(UniqueViolation, _)));
        Ok(())
    });

    connection.test_transaction(|| query.execute(&connection));
}

#[test]
fn foreign_key_violation_detected() {
    let connection = connection();

    let failure = insert_into(fk_tests::table)
        .values(&FkTest::new(1, 100))
        .execute(&connection);
    assert_matches!(failure, Err(DatabaseError(ForeignKeyViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn foreign_key_violation_correct_constraint_name() {
    let connection = connection();

    let failure = insert_into(fk_tests::table)
        .values(&FkTest::new(1, 100))
        .execute(&connection);
    match failure {
        Err(DatabaseError(ForeignKeyViolation, e)) => {
            assert_eq!(Some("fk_tests"), e.table_name());
            assert_eq!(None, e.column_name());
            assert_eq!(Some("fk_tests_fk_id_fkey"), e.constraint_name());
        }
        _ => panic!(
            "{:?} did not match Err(DatabaseError(ForeignKeyViolation, e))",
            failure
        ),
    }
}

#[test]
#[cfg(feature = "postgres")]
fn isolation_errors_are_detected() {
    use diesel::result::DatabaseErrorKind::SerializationFailure;
    use diesel::result::Error::DatabaseError;
    use std::sync::{Arc, Barrier};
    use std::thread;

    table! {
        #[sql_name = "isolation_errors_are_detected"]
        isolation_example {
            id -> Serial,
            class -> Integer,
        }
    }

    let conn = connection_without_transaction();

    sql_query("DROP TABLE IF EXISTS isolation_errors_are_detected;")
        .execute(&conn)
        .unwrap();
    sql_query(
        r#"
        CREATE TABLE isolation_errors_are_detected (
            id SERIAL PRIMARY KEY,
            class INTEGER NOT NULL
        )
    "#,
    )
    .execute(&conn)
    .unwrap();

    insert_into(isolation_example::table)
        .values(&vec![
            isolation_example::class.eq(1),
            isolation_example::class.eq(2),
        ])
        .execute(&conn)
        .unwrap();

    let barrier = Arc::new(Barrier::new(2));
    let threads = (1..3)
        .map(|i| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                let conn = connection_without_transaction();

                conn.build_transaction().serializable().run(|| {
                    let _ = isolation_example::table
                        .filter(isolation_example::class.eq(i))
                        .count()
                        .execute(&conn)?;

                    barrier.wait();

                    let other_i = if i == 1 { 2 } else { 1 };
                    insert_into(isolation_example::table)
                        .values(isolation_example::class.eq(other_i))
                        .execute(&conn)
                })
            })
        })
        .collect::<Vec<_>>();

    let mut results = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Vec<_>>();

    results.sort_by_key(|r| r.is_err());

    assert_matches!(results[0], Ok(_));
    assert_matches!(results[1], Err(DatabaseError(SerializationFailure, _)));
}
