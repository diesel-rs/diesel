pub fn table_exists(url: &str, table: &str) -> bool {
    use diesel::pg::PgConnection;
    use diesel::expression::sql;
    use diesel::types::Bool;
    use diesel::{Connection, select, LoadDsl};
    let conn = PgConnection::establish(url).unwrap();

    select(sql::<Bool>(&format!("EXISTS \
            (SELECT 1 \
             FROM information_schema.tables \
             WHERE table_name = '{}')", table)))
        .get_result(&conn).unwrap()
}

pub fn database_exists(url: &str) -> bool {
    use diesel::pg::PgConnection;
    use diesel::prelude::Connection;
    PgConnection::establish(url).is_ok()
}
