extern crate diesel;
use diesel::pg::CopyFormat;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value = false)]
struct NewUser {
    name: &'static str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
struct User {
    name: String,
}

fn main() {
    let conn = &mut PgConnection::establish("_").unwrap();

    // that works
    diesel::copy_from(users::table)
        .from_insertable(vec![NewUser { name: "John" }])
        .execute(conn)
        .unwrap();

    diesel::copy_from(users::table)
        .from_raw_data(users::table, |copy| diesel::QueryResult::Ok(()))
        .with_format(CopyFormat::Csv)
        .execute(conn)
        .unwrap();

    diesel::copy_to(users::table).load::<User, _>(conn).unwrap();
    diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .load_raw(conn)
        .unwrap();

    // that fails
    diesel::copy_from(users::table)
        .from_insertable(vec![NewUser { name: "John" }])
        .with_format(CopyFormat::Csv)
        .execute(conn)
        .unwrap();

    diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .load::<User, _>(conn)
        .unwrap();
}
