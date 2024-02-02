use crate::schema::*;
use diesel::deserialize::Queryable;
use diesel::prelude::*;
use diesel::sql_query;
use std::borrow::Cow;

#[derive(Queryable, PartialEq, Debug, Selectable)]
#[diesel(table_name = users)]
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
        users.select((id, name)).order(id).first(connection)
    );
    assert_eq!(
        users
            .select((id, name))
            .order(id)
            .first::<CowUser<'_>>(connection),
        users
            .select(CowUser::as_select())
            .order(id)
            .first(connection)
    );
}

struct CustomString(String);

#[derive(Selectable)]
#[diesel(table_name = my_users)]
pub struct User {
    id: i32,
    username: CustomString,
    password_hash: [u8; 3],
}

diesel::table! {
    my_users (id) {
        id -> Integer,
        username -> Varchar,
        password_hash -> Binary,
    }
}

impl Queryable<my_users::SqlType, TestBackend> for User {
    type Row = (i32, *const str, *const [u8]);
    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(Self {
            id: row.0,
            username: CustomString(unsafe { row.1.as_ref().unwrap().into() }),
            password_hash: unsafe { row.2.as_ref().unwrap().try_into().unwrap() },
        })
    }
}

#[test]
fn check_deserialize_composite_ptr_types() {
    let conn = &mut connection();

    #[cfg(not(feature = "postgres"))]
    sql_query(
        "CREATE TEMPORARY TABLE my_users(\
               id INTEGER NOT NULL PRIMARY KEY, \
               username TEXT NOT NULL, \
               password_hash BINARY(3) NOT NULL)",
    )
    .execute(conn)
    .unwrap();

    #[cfg(feature = "postgres")]
    sql_query(
        "CREATE TEMPORARY TABLE my_users(\
               id INTEGER NOT NULL PRIMARY KEY, \
               username TEXT NOT NULL, \
               password_hash BYTEA NOT NULL)",
    )
    .execute(conn)
    .unwrap();

    diesel::insert_into(my_users::table)
        .values((
            my_users::id.eq(42),
            my_users::username.eq("John"),
            my_users::password_hash.eq(b"abc".to_vec()),
        ))
        .execute(conn)
        .unwrap();

    let r = my_users::table
        .select(User::as_select())
        .load(conn)
        .unwrap();

    assert_eq!(r.len(), 1);
    assert_eq!(r[0].id, 42);
    assert_eq!(r[0].username.0, "John");
    assert_eq!(&r[0].password_hash, b"abc");
}
