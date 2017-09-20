#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

pub struct NewUser(String);

impl_Insertable! {
    (users)
    pub struct NewUser(#[column_name(name)] String,);
}

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

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .returning(non_users::columns::noname);
    //~^ ERROR SelectableExpression

    update(users::table)
        .set(users::columns::name.eq("Bill"))
        .returning(non_users::columns::noname);
    //~^ ERROR SelectableExpression
}
