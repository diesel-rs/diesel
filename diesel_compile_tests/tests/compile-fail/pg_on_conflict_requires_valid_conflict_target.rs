#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

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
#[table_name="users"]
struct NewUser(#[column_name(name)] &'static str);

sql_function!(lower, lower_t, (x: diesel::types::Text) -> diesel::types::Text);

fn main() {
    use self::users::dsl::*;
    let connection = PgConnection::establish("postgres://localhost").unwrap();

    let valid_insert = insert_into(users).values(&NewUser("Sean").on_conflict(id, do_nothing())).execute(&connection);
    // Sanity check, no error

    let column_from_other_table = insert_into(users)
        .values(&NewUser("Sean").on_conflict(posts::id, do_nothing()));
        //~^ ERROR type mismatch resolving `<posts::columns::id as diesel::Column>::Table == users::table`

    let expression_using_column_from_other_table = insert_into(users)
        .values(&NewUser("Sean").on_conflict(lower(posts::title), do_nothing()));
        //~^ ERROR the trait bound `lower_t<posts::columns::title>: diesel::Column` is not satisfied

    let random_non_expression = insert_into(users)
        .values(&NewUser("Sean").on_conflict("id", do_nothing()));
        //~^ ERROR the trait bound `&str: diesel::Column` is not satisfied
}
