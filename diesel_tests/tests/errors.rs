use diesel;
use diesel::prelude::*;
use diesel::result::Error::DatabaseError;
use diesel::result::DatabaseErrorKind::{ForeignKeyViolation, UniqueViolation};
use schema::*;

#[test]
fn unique_constraints_are_detected() {
    let connection = connection();
    diesel::insert(&User::new(1, "Sean"))
        .into(users::table)
        .execute(&connection)
        .unwrap();

    let failure = diesel::insert(&User::new(1, "Jim"))
        .into(users::table)
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
    diesel::insert(&User::new(1, "Sean"))
        .into(users::table)
        .execute(&connection)
        .unwrap();

    let failure = diesel::insert(&User::new(2, "Sean"))
        .into(users::table)
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
    let query = diesel::insert(&user).into(users::table);

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

    let failure = diesel::insert(&FkTest::new(1, 100))
        .into(fk_tests::table)
        .execute(&connection);
    assert_matches!(failure, Err(DatabaseError(ForeignKeyViolation, _)));
}

#[test]
#[cfg(feature = "postgres")]
fn foreign_key_violation_correct_constraint_name() {
    let connection = connection();

    let failure = diesel::insert(&FkTest::new(1, 100))
        .into(fk_tests::table)
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
