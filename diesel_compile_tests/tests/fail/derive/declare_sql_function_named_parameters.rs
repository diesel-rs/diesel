extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;

#[diesel::declare_sql_function(named_parameters = true)]
extern "SQL" {
    fn foo(a: Text);
}

fn main() {
    // Named parameters are supported by Postgres

    let mut connection = PgConnection::establish("…").unwrap();
    diesel::select(foo("a")).execute(&mut connection).unwrap();

    // Named parameters are not supported by Mysql, Mariadb, and Sqlite

    let mut connection = SqliteConnection::establish("…").unwrap();
    diesel::select(foo("a")).execute(&mut connection).unwrap();
    //~^ ERROR: `diesel::internal::sql_functions::FunctionParameter<true>` is no valid SQL fragment for the `Sqlite` backend

    let mut connection = MysqlConnection::establish("…").unwrap();
    diesel::select(foo("a")).execute(&mut connection).unwrap();
    //~^ ERROR: `diesel::internal::sql_functions::FunctionParameter<true>` is no valid SQL fragment for the `Mysql` backend

    let mut connection = MariadbConnection::establish("…").unwrap();
    diesel::select(foo("a")).execute(&mut connection).unwrap();
    //~^ ERROR: `diesel::internal::sql_functions::FunctionParameter<true>` is no valid SQL fragment for the `Mariadb` backend
}
