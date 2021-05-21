extern crate diesel;

use diesel::*;
use diesel::sql_types::*;
use diesel::dsl::*;
use diesel::upsert::on_constraint;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

sql_function!(fn lower(x: VarChar) -> VarChar);

#[derive(Insertable)]
#[table_name="users"]
struct NewUser(#[column_name = "name"] &'static str);

// NOTE: This test is meant to be comprehensive, but not exhaustive.
fn main() {
    use self::users::dsl::*;
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    users.select(id).filter(name.eq(any(Vec::<String>::new())))
        .load::<i32>(&mut connection);
    users.select(id).filter(name.is_not_distinct_from("Sean"))
        .load::<i32>(&mut connection);
    users.select(id).filter(now.eq(now.at_time_zone("UTC")))
        .load::<i32>(&mut connection);
    insert_into(users).values(&NewUser("Sean"))
        .on_conflict(on_constraint("name"))
        .execute(&mut connection);
}
