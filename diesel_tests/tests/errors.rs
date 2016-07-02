use diesel;
use diesel::prelude::*;
use diesel::result::Error::DatabaseError;
use schema::*;

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
        assert_matches!(failure, Err(DatabaseError(_)));
        Ok(())
    });

    connection.test_transaction(|| query.execute(&connection));
}
