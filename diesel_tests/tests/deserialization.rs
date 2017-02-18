use schema::*;
use diesel::*;
use std::borrow::Cow;

#[derive(Queryable, PartialEq, Debug)]
struct CowUser<'a> {
    id: i32,
    name: Cow<'a, str>,
}

#[test]
fn generated_queryable_allows_lifetimes() {
    use schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_user = CowUser {
        id: 1,
        name: Cow::Owned("Sean".to_string()),
    };
    assert_eq!(Ok(expected_user), users.select(hlist!(id, name)).first(&connection));
}
