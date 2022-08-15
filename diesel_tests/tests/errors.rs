use crate::schema::*;
#[cfg(not(feature = "mysql"))]
use diesel::result::DatabaseErrorKind::CheckViolation;
use diesel::result::DatabaseErrorKind::{ForeignKeyViolation, NotNullViolation, UniqueViolation};
use diesel::result::Error::DatabaseError;
use diesel::*;

#[test]
fn unique_constraints_are_detected() {
    let connection = &mut connection();
    insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .execute(connection)
        .unwrap();

    let failure = insert_into(users::table)
        .values(&User::new(1, "Jim"))
        .execute(connection);
    assert_matches!(failure, Err(DatabaseError(UniqueViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn unique_constraints_report_correct_constraint_name() {
    let connection = &mut connection();
    diesel::sql_query("CREATE UNIQUE INDEX users_name ON users (name)")
        .execute(connection)
        .unwrap();
    insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .execute(connection)
        .unwrap();

    let failure = insert_into(users::table)
        .values(&User::new(2, "Sean"))
        .execute(connection);
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
    let connection = &mut connection_without_transaction();
    let user = User::new(1, "Sean");
    let query = insert_into(users::table).values(&user);

    connection.test_transaction(|connection| {
        try_no_coerce!(query.execute(connection));

        let failure = query.execute(connection);
        assert_matches!(failure, Err(DatabaseError(UniqueViolation, _)));
        Ok(())
    });

    connection.test_transaction(|connection| query.execute(connection));
}

#[test]
fn foreign_key_violation_detected() {
    let connection = &mut connection();

    let failure = insert_into(fk_tests::table)
        .values(&FkTest::new(1, 100))
        .execute(connection);
    assert_matches!(failure, Err(DatabaseError(ForeignKeyViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn foreign_key_violation_correct_constraint_name() {
    let connection = &mut connection();

    let failure = insert_into(fk_tests::table)
        .values(&FkTest::new(1, 100))
        .execute(connection);
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
// This is a false positive as there is a side effect of this collect (spawning threads)
#[allow(clippy::needless_collect)]
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

    let conn = &mut connection_without_transaction();

    sql_query("DROP TABLE IF EXISTS isolation_errors_are_detected;")
        .execute(conn)
        .unwrap();
    sql_query(
        r#"
        CREATE TABLE isolation_errors_are_detected (
            id SERIAL PRIMARY KEY,
            class INTEGER NOT NULL
        )
    "#,
    )
    .execute(conn)
    .unwrap();

    insert_into(isolation_example::table)
        .values(&vec![
            isolation_example::class.eq(1),
            isolation_example::class.eq(2),
        ])
        .execute(conn)
        .unwrap();

    let before_barrier = Arc::new(Barrier::new(2));
    let after_barrier = Arc::new(Barrier::new(2));
    let threads = (1..3)
        .map(|i| {
            let before_barrier = before_barrier.clone();
            let after_barrier = after_barrier.clone();
            thread::spawn(move || {
                let conn = &mut connection_without_transaction();

                conn.build_transaction().serializable().run(|conn| {
                    let _ = isolation_example::table
                        .filter(isolation_example::class.eq(i))
                        .count()
                        .execute(conn)?;

                    before_barrier.wait();

                    let other_i = if i == 1 { 2 } else { 1 };
                    let r = insert_into(isolation_example::table)
                        .values(isolation_example::class.eq(other_i))
                        .execute(conn);
                    after_barrier.wait();
                    r
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

#[test]
#[cfg(not(feature = "sqlite"))]
fn read_only_errors_are_detected() {
    use diesel::connection::SimpleConnection;
    use diesel::result::DatabaseErrorKind::ReadOnlyTransaction;

    let conn = &mut connection_without_transaction();
    conn.batch_execute("START TRANSACTION READ ONLY").unwrap();

    let result = users::table.for_update().load::<User>(conn);

    assert_matches!(result, Err(DatabaseError(ReadOnlyTransaction, _)));
}

#[test]
fn not_null_constraints_are_detected() {
    let connection = &mut connection();

    let failure = insert_into(users::table)
        .values(users::columns::hair_color.eq("black"))
        .execute(connection);

    assert_matches!(failure, Err(DatabaseError(NotNullViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn not_null_constraints_correct_column_name() {
    let connection = &mut connection();

    let failure = insert_into(users::table)
        .values(users::columns::hair_color.eq("black"))
        .execute(connection);

    match failure {
        Err(DatabaseError(NotNullViolation, e)) => {
            assert_eq!(Some("users"), e.table_name());
            assert_eq!(Some("name"), e.column_name());
        }
        _ => panic!(
            "{:?} did not match Err(DatabaseError(NotNullViolation, e))",
            failure
        ),
    };
}

#[test]
#[cfg(not(feature = "mysql"))]
/// MySQL < 8.0.16 doesn't enforce check constraints
fn check_constraints_are_detected() {
    let connection = &mut connection();

    insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .execute(connection)
        .unwrap();

    let failure = insert_into(pokes::table)
        .values((
            pokes::columns::user_id.eq(1),
            pokes::columns::poke_count.eq(-1),
        ))
        .execute(connection);

    assert_matches!(failure, Err(DatabaseError(CheckViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn check_constraints_correct_constraint_name() {
    let connection = &mut connection();

    insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .execute(connection)
        .unwrap();

    let failure = insert_into(pokes::table)
        .values((
            pokes::columns::user_id.eq(1),
            pokes::columns::poke_count.eq(-1),
        ))
        .execute(connection);

    match failure {
        Err(DatabaseError(CheckViolation, e)) => {
            assert_eq!(Some("pokes"), e.table_name());
            assert_eq!(None, e.column_name());
            assert_eq!(Some("pokes_poke_count_check"), e.constraint_name());
        }
        _ => panic!(
            "{:?} did not match Err(DatabaseError(CheckViolation, e))",
            failure
        ),
    };
}
