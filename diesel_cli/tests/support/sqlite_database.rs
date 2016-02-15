pub fn database_exists(url: &str) -> bool {
    use std::path::Path;
    Path::new(url).exists()
}

pub fn table_exists(url: &str, table: &str) -> bool {
    use diesel::sqlite::SqliteConnection;
    use diesel::{Connection, select, LoadDsl};
    use diesel::expression::sql;
    use diesel::types::Bool;
    let conn = SqliteConnection::establish(url).unwrap();

    select(sql::<Bool>(&format!("EXISTS \
            (SELECT 1 \
             FROM sqlite_master \
             WHERE type = 'table' \
             AND name = '{}')", table)))
        .get_result(&conn).unwrap()
}
