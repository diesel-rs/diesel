#[macro_use]
extern crate yaqb;

use yaqb::*;
use yaqb::expression::count;

table! {
    users {
        id -> Serial,
    }
}

fn main() {
    use self::users::columns::*;
    use self::users::table as users;

    let connection = Connection::establish("").unwrap();
    let source = users.select((id, count(star)));
    //~^ ERROR E0277
}
