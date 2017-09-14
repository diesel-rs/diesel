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

    let valid_insert = insert(&NewUser("Sean").on_conflict(id, do_nothing())).into(users).execute(&connection);
    // Sanity check, no error

    // Using UFCS to get a more specific error message
    let column_from_other_table = <_ as ExecuteDsl<_>>::execute(
        //~^ ERROR type mismatch resolving `<posts::columns::id as diesel::Column>::Table == users::table`
        insert(&NewUser("Sean").on_conflict(posts::id, do_nothing())).into(users),
        &connection,
    );

    let expression_using_column_from_other_table = <_ as ExecuteDsl<_>>::execute(
        //~^ ERROR E0277
        insert(&NewUser("Sean").on_conflict(lower(posts::title), do_nothing())).into(users),
        &connection,
    );

    let random_non_expression = <_ as ExecuteDsl<_>>::execute(
        //~^ ERROR E0277
        insert(&NewUser("Sean").on_conflict("id", do_nothing())).into(users),
        &connection,
    );
}
