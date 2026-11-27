extern crate diesel;

use diesel::sqlite::SqliteFunctionBehavior;
use diesel::*;

view! {
    grantees {
        name -> Text,
        role -> Text,
    }
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    connection.register_table_valued_function(
    //~^ ERROR: `(&str, {integer})` is no row of the result columns typed `(diesel::sql_types::Text,)`
        (grantees::name,),
        (grantees::role,),
        SqliteFunctionBehavior::empty(),
        |_, _: (String,)| Ok(vec![("alice", 1)]),
    );

    connection.register_table_valued_function(
    //~^ ERROR: the trait bound `{integer}: ToSql<diesel::sql_types::Text, Sqlite>` is not satisfied
        (grantees::name,),
        (grantees::role,),
        SqliteFunctionBehavior::empty(),
        |_, _: (String,)| Ok(vec![(1,)]),
    );

    connection.register_table_valued_function(
        (grantees::name,),
        (grantees::role,),
        //~^ ERROR: the trait bound `(i32,): FromStaticSqlRow<(diesel::sql_types::Text,), Sqlite>` is not satisfied
        SqliteFunctionBehavior::empty(),
        |_, _: (i32,)| Ok(Vec::<(String,)>::new()),
    );
}
