use schema::{connection, NewUser, setup_users_table};
use schema::users::columns::*;
use schema::users::table as users;
use yaqb::QuerySource;
use yaqb::expression::*;

#[test]
fn test_count_counts_the_rows() {
    let connection = connection();
    setup_users_table(&connection);
    let source = users.select(count(star));

    assert_eq!(Some(0), connection.query_one(&source).unwrap());
    connection.insert_without_return(&users, vec![NewUser::new("Sean", None)]).unwrap();
    assert_eq!(Some(1), connection.query_one(&source).unwrap());
}
