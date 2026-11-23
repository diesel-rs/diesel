extern crate diesel;

use diesel::sqlite::SqliteFunctionBehavior;
use diesel::*;

table! {
    grants (id) {
        id -> Integer,
        role -> Text,
    }
}

view! {
    grantees {
        name -> Text,
        role -> Text,
    }
}

view! {
    owners {
        role -> Text,
    }
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    connection.register_table_valued_function(
        (grantees::name,),
        (owners::role,),
        //~^ ERROR: type mismatch resolving `<role as QueryRelationField>::QueryRelation == view`
        SqliteFunctionBehavior::empty(),
        |_, _: (String,)| Ok(Vec::<(String,)>::new()),
    );

    connection.register_table_valued_function(
        (grants::role,),
        //~^ ERROR: the trait bound `grants::table: View` is not satisfied
        (grants::id,),
        SqliteFunctionBehavior::empty(),
        |_, _: (i32,)| Ok(Vec::<(String,)>::new()),
    );
}
