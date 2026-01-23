extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    use diesel::dsl::*;

    let pg_connection = &mut PgConnection::establish("…").unwrap();
    let mysql_connection = &mut MysqlConnection::establish("…").unwrap();

    let query = users::table.select(json(users::name));

    let _ = query.execute(pg_connection);
    //~^ ERROR: `diesel::sqlite::expression::functions::json_utils::json<diesel::sql_types::Text, columns::name>` is no valid SQL fragment for the `Pg` backend
    let _ = query.execute(mysql_connection);
    //~^ ERROR: `diesel::sqlite::expression::functions::json_utils::json<diesel::sql_types::Text, columns::name>` is no valid SQL fragment for the `Mysql` backend
}
