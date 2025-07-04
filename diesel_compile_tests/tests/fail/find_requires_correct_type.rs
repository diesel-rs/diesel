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
    let mut connection = PgConnection::establish("").unwrap();
    // FIXME: It'd be nice if this mentioned `AsExpression`
    int_primary_key::table.find("1");
    //~^ ERROR: the trait bound `str: diesel::Expression` is not satisfied
    //~| ERROR: the trait bound `str: ValidGrouping<()>` is not satisfied
    // FIXME: It'd be nice if this mentioned `AsExpression`
    string_primary_key::table.find(1);
    //~^ ERROR: the trait bound `{integer}: diesel::Expression` is not satisfied
    //~| ERROR: the trait bound `{integer}: ValidGrouping<()>` is not satisfied
}
