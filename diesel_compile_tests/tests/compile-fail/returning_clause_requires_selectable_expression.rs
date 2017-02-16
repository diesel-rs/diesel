#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser(
    #[column_name(name)]
    String
);

table! {
    non_users {
        id -> Integer,
        noname -> VarChar,
    }
}

fn main() {
    let connection = PgConnection::establish("").unwrap();

    delete(users::table.filter(users::columns::name.eq("Bill")))
        .returning(non_users::columns::noname);
    //~^ ERROR SelectableExpression

    insert(&NewUser("Hello".into()))
        .into(users::table)
        .returning(non_users::columns::noname);
    //~^ ERROR SelectableExpression

    update(users::table)
        .set(users::columns::name.eq("Bill"))
        .returning(non_users::columns::noname);
    //~^ ERROR SelectableExpression
}
