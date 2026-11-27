extern crate diesel;

use diesel::sql_types::Text;
use diesel::*;

view! {
    grantees {
        name -> Text,
    }
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(grantees)]
    fn grantees(role: Text) -> grantees::SqlType;
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    grantees_utils::register_impl(&mut connection, |_, _: String| Ok(vec![("alice", 1)]));
    //~^ ERROR: `(&str, {integer})` is no row of the result columns typed `(diesel::sql_types::Text,)`

    grantees_utils::register_impl(&mut connection, |_, _: String| Ok(vec![(1,)]));
    //~^ ERROR: the trait bound `{integer}: ToSql<diesel::sql_types::Text, Sqlite>` is not satisfied

    grantees_utils::register_impl(&mut connection, |_, _: i32| Ok(Vec::<(String,)>::new()));
    //~^ ERROR: cannot deserialize a value of the database type `diesel::sql_types::Text` as `i32`
}
