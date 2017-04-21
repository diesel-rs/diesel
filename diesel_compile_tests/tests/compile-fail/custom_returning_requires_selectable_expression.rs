#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    bad {
      id -> Integer,
      age -> Integer,
    }
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser {
    name: String,
}

fn main() {
    use self::users::dsl::*;

    let stmt = update(users.filter(id.eq(1))).set(name.eq("Bill")).returning(bad::age);
    //~^ ERROR SelectableExpression

    let new_user = NewUser {
        name: "Foobar".to_string(),
    };
    let stmt = insert(&new_user).into(users).returning((name, bad::age));
    //~^ ERROR SelectableExpression
    //~| ERROR AppearsOnTable
}
