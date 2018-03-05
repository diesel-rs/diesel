#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    int_primary_key {
        id -> Integer,
    }
}

table! {
    string_primary_key {
        id -> VarChar,
    }
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    // FIXME: It'd be nice if this mentioned `AsExpression`
    int_primary_key::table.find("1");
    //~^ ERROR Expression
    //~| ERROR E0277
}
