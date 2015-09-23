#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Serial,
        title -> VarChar,
    }
}

fn main() {
    let connection = Connection::establish("").unwrap();
    let select_id = users::table.select(posts::id);
    //~^ ERROR SelectableColumn
    let select_id = users::table.select(posts::id);
    //~^ ERROR E0277
}
