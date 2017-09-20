use diesel::*;
use diesel::result::Error::DatabaseError;
use diesel::result::DatabaseErrorKind::{ForeignKeyViolation, UniqueViolation};
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
    ($e:expr) => ({
        match $e {
            Ok(e) => e,
            Err(e) => return Err(e),
        }
    })
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
