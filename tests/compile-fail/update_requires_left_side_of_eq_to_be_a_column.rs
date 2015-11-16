#[macro_use]
extern crate yaqb;

use yaqb::*;
use yaqb::expression::AsExpression;
use yaqb::query_builder::update;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let foo = AsExpression::<types::VarChar>::as_expression("foo");
    let command = update(users).set(foo.eq(name));
    //~^ ERROR Column
}
