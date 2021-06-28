use crate::schema::*;
use diesel::deserialize::FromSqlRow;
use diesel::prelude::*;
use std::borrow::Cow;

#[derive(Queryable, PartialEq, Debug, Selectable)]
#[table_name = "users"]
struct CowUser<'a> {
    id: i32,
    name: Cow<'a, str>,
}

#[test]
fn generated_queryable_allows_lifetimes() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let expected_user = CowUser {
        id: 1,
        name: Cow::Owned("Sean".to_string()),
    };
    assert_eq!(
        Ok(expected_user),
        users.select((id, name)).first(connection)
    );
    assert_eq!(
        users.select((id, name)).first::<CowUser>(connection),
        users.select(CowUser::as_select()).first(connection)
    );
}
