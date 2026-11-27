extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;

view! {
    grantees {
        name -> Text,
    }
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(grantees)]
    #[sql_name = "main.grantees"]
    fn grantees(role: Text) -> grantees::SqlType;
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    grantees_utils::register_impl(&mut connection, |_, _: String| Ok(Vec::<(String,)>::new()));
    //~^ ERROR: cannot find function `register_impl` in module `grantees_utils`
}
