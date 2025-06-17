extern crate diesel;

use diesel::dsl::*;
use diesel::sql_types::*;
use diesel::upsert::on_constraint;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[declare_sql_function]
extern "SQL" {
    fn lower(x: VarChar) -> VarChar;
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser(#[diesel(column_name = name)] &'static str);

// NOTE: This test is meant to be comprehensive, but not exhaustive.
fn main() {
    use self::users::dsl::*;
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    users
        .select(id)
        .filter(name.eq(any(Vec::<String>::new())))
        .load::<i32>(&mut connection);
    //~^ ERROR: `Any<Bound<Array<Text>, Vec<String>>>` is no valid SQL fragment for the `Sqlite` backend
    users
        .select(id)
        .filter(name.is_not_distinct_from("Sean"))
        .load::<i32>(&mut connection);
    //~^ ERROR: `IsNotDistinctFrom<name, Bound<Text, &str>>` is no valid SQL fragment for the `Sqlite` backend
    users
        .select(id)
        .filter(now.eq(now.at_time_zone("UTC")))
        .load::<i32>(&mut connection);
    //~^ ERROR: `AtTimeZone<now, Bound<Text, &str>>` is no valid SQL fragment for the `Sqlite` backend
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(on_constraint("name"))
        .execute(&mut connection);
    //~^ ERROR: the method `execute` exists for struct `IncompleteOnConflict<InsertStatement<table, ...>, ...>`, but its trait bounds were not satisfied
}
