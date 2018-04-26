#[macro_use] extern crate diesel;

use diesel::*;
use diesel::pg::upsert::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        title -> VarChar,
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser(#[column_name = "name"] &'static str);

sql_function!(fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text);

fn main() {
    use self::users::dsl::*;
    let connection = PgConnection::establish("postgres://localhost").unwrap();

    let valid_insert = insert_into(users).values(&NewUser("Sean")).on_conflict(id).do_nothing().execute(&connection);
    // Sanity check, no error

    let column_from_other_table = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(posts::id);
        //~^ ERROR type mismatch resolving `<posts::columns::id as diesel::Column>::Table == users::table`

    let expression_using_column_from_other_table = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(lower(posts::title));
        //~^ ERROR the trait bound `lower::lower<posts::columns::title>: diesel::Column` is not satisfied

    let random_non_expression = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict("id");
        //~^ ERROR the trait bound `&str: diesel::Column` is not satisfied
}
