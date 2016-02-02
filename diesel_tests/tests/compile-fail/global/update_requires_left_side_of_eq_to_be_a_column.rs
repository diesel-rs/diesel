#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::AsExpression;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let foo = AsExpression::<types::VarChar>::as_expression("foo");
    let command = update(users).set(foo.eq(name));
    //~^ ERROR Column
}
