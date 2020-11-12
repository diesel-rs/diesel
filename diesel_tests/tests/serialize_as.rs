use crate::schema::*;
use diesel::backend::Backend;
use diesel::serialize::{Output, ToSql};
use diesel::*;
use std::io::Write;

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
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

#[derive(Insertable, PartialEq, Debug)]
#[diesel(table_name = users)]
struct InsertableUser {
    #[diesel(serialize_as = UppercaseString)]
    name: String,
}

#[test]
fn serialization_can_be_customized() {
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
