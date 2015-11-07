#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    int_primary_key {
        id -> Serial,
    }
}

table! {
    string_primary_key {
        id -> VarChar,
    }
}

fn main() {
    let connection = Connection::establish("").unwrap();
    let one = connection.find(int_primary_key::table, &"1".to_string()).unwrap();
    //~^ ERROR the trait `yaqb::types::ToSql<yaqb::types::Serial>` is not implemented
    let string = connection.find(string_primary_key::table, &1).unwrap();
    //~^ ERROR the trait `yaqb::types::ToSql<yaqb::types::VarChar>` is not implemented
}
