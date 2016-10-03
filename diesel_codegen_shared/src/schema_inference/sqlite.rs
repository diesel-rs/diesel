use diesel::*;
use diesel::expression::dsl::sql;
use diesel::sqlite::SqliteConnection;
use std::error::Error;

table! {
    sqlite_master (name) {
        name -> VarChar,
    }
}

pub fn load_table_names(connection: &SqliteConnection) -> Result<Vec<String>, Box<Error>> {
    use self::sqlite_master::dsl::*;

    sqlite_master.select(name)
        .filter(name.not_like("\\_\\_%").escape('\\'))
        .filter(name.not_like("sqlite%"))
        .filter(sql("type='table'"))
        .load(connection)
        .map_err(Into::into)
}
