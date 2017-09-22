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

#[derive(Insertable)]
#[table_name="users"]
struct NewUser(#[column_name(name)] &'static str);

fn main() {
    use self::users::dsl::*;
    let connection = PgConnection::establish("postgres://localhost").unwrap();

    insert_into(users).values(&NewUser("Sean").on_conflict_do_nothing().on_conflict_do_nothing()).execute(&connection);
    //~^ ERROR E0277
    insert_into(users).values(&NewUser("Sean").on_conflict(id, do_nothing()).on_conflict_do_nothing()).execute(&connection);
    //~^ ERROR E0277
    insert_into(users).values(&NewUser("Sean").on_conflict_do_nothing().on_conflict(id, do_nothing())).execute(&connection);
    //~^ ERROR E0277
    insert_into(users).values(&NewUser("Sean").on_conflict(id, do_nothing()).on_conflict(id, do_nothing())).execute(&connection);
    //~^ ERROR E0277
    insert_into(users).values(&vec![NewUser("Sean").on_conflict_do_nothing()]).execute(&connection);
    //~^ ERROR E0277
    insert_into(users).values(&vec![&NewUser("Sean").on_conflict_do_nothing()]).execute(&connection);
    //~^ ERROR no method named `execute`
    //~| ERROR E0277
    insert_into(users).values(&vec![&NewUser("Sean").on_conflict(id, do_nothing())]).execute(&connection);
    //~^ ERROR no method named `execute`
    //~| ERROR E0277
    insert_into(users).values(&(name.eq("Sean").on_conflict_do_nothing(),)).execute(&connection);
    //~^ ERROR E0277
}
