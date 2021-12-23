use crate::schema::*;
use diesel::backend::Backend;
use diesel::serialize::{Output, ToSql};
use diesel::*;

#[derive(Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
struct UppercaseString(pub String);

impl From<String> for UppercaseString {
    fn from(s: String) -> Self {
        UppercaseString(s.to_uppercase())
    }
}

impl<DB> ToSql<sql_types::Text, DB> for UppercaseString
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[diesel(table_name = users)]
struct InsertableUser {
    #[diesel(serialize_as = UppercaseString)]
    name: String,
}

#[derive(Clone, Debug, AsChangeset, Identifiable)]
#[diesel(table_name = users)]
struct ChangeUser {
    id: i32,
    #[diesel(serialize_as = UppercaseString)]
    name: String,
}

#[test]
fn insert_serialization_can_be_customized() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let user = InsertableUser {
        name: "thomas".to_string(),
    };

    diesel::insert_into(users)
        .values(user)
        .execute(connection)
        .unwrap();

    assert_eq!(
        Ok("THOMAS".to_string()),
        users.select(name).first(connection)
    );
}

#[test]
fn update_serialization_can_be_customized() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let user = InsertableUser {
        name: "thomas".to_string(),
    };
    diesel::insert_into(users)
        .values(user)
        .execute(connection)
        .unwrap();

    let user = ChangeUser {
        id: users.select(id).first(connection).unwrap(),
        name: "eizinger".to_string(),
    };
    diesel::update(&user)
        .set(user.clone())
        .execute(connection)
        .unwrap();

    assert_eq!(
        Ok("EIZINGER".to_string()),
        users.select(name).first(connection)
    );
}
