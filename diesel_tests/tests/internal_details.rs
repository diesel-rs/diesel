use schema::*;
use diesel::types::*;
use diesel::dsl::sql;
use diesel::*;

#[test]
#[cfg(not(feature = "mysql"))] // ? IS NULL is invalid syntax for MySQL
fn bind_params_are_passed_for_null_when_not_inserting() {
    let connection = connection();
    let query =
        select(sql::<Integer>("1")).filter(None::<i32>.into_sql::<Nullable<Integer>>().is_null());
    assert_eq!(Ok(1), query.first(&connection));
}

#[test]
#[cfg(feature = "postgres")]
fn query_which_cannot_be_transmitted_gives_proper_error_message() {
    use schema::comments::dsl::*;
    use diesel::result::Error::DatabaseError;
    use diesel::result::DatabaseErrorKind::UnableToSendCommand;

    // Create a query with 90000 binds, 2 binds per row
    let data: &[NewComment<'static>] = &[NewComment(1, "hi"); 45_000];
    let query_with_to_many_binds = insert_into(comments).values(data);

    match query_with_to_many_binds.execute(&connection()) {
        Ok(_) => panic!(
            "We successfully executed a query with 90k binds. \
             We need to find a new query to test which can't be represented by \
             the wire protocol."
        ),
        Err(DatabaseError(UnableToSendCommand, info)) => assert_ne!("", info.message()),
        Err(_) => panic!("We got back the wrong kind of error. This test is invalid."),
    }
}

#[test]
#[cfg(feature = "postgres")]
fn empty_query_gives_proper_error_instead_of_panicking() {
    use diesel::result::Error::DatabaseError;
    use diesel::result::DatabaseErrorKind::__Unknown;
    use diesel::dsl::sql;

    let connection = connection();
    let query = sql::<Integer>("");

    match query.execute(&connection) {
        Ok(_) => panic!("We successfully executed an empty query"),
        Err(DatabaseError(__Unknown, info)) => assert_ne!("", info.message()),
        Err(_) => panic!("We got back the wrong kind of error. This test is invalid."),
    }
}
