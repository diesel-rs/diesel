use crate::schema::*;
use diesel::*;
use std::borrow::Cow;

#[derive(Queryable, PartialEq, Debug, QueryableByColumn)]
#[table_name = "users"]
struct CowUser<'a> {
    id: i32,
    name: Cow<'a, str>,
}

#[test]
fn generated_queryable_allows_lifetimes() {
    use crate::schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_user = CowUser {
        id: 1,
        name: Cow::Owned("Sean".to_string()),
    };
    assert_eq!(
        Ok(expected_user),
        users.select_by::<CowUser>().first(&connection)
    );
}
