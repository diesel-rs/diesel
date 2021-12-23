#[cfg(feature = "postgres")]
pub fn create_user_table(conn: &mut diesel::PgConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users (id Serial PRIMARY KEY, name TEXT NOT NULL DEFAULT '', hair_color TEXT)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "sqlite")]
pub fn create_user_table(conn: &mut diesel::SqliteConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL DEFAULT '', hair_color TEXT)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "mysql")]
pub fn create_user_table(conn: &mut diesel::MysqlConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTO_INCREMENT, name TEXT NOT NULL, hair_color TEXT)")
        .execute(conn)
        .unwrap();
    diesel::sql_query("DELETE FROM users")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "sqlite")]
pub fn establish_connection() -> diesel::SqliteConnection {
    use diesel::*;

    SqliteConnection::establish(":memory:").unwrap()
}

#[cfg(feature = "postgres")]
pub fn establish_connection() -> diesel::PgConnection {
    use diesel::*;

    let mut conn = PgConnection::establish(
        &dotenv::var("DATABASE_URL")
            .or_else(|_| dotenv::var("PG_DATABASE_URL"))
            .expect("Set either `DATABASE_URL` or `PG_DATABASE_URL`"),
    )
    .unwrap();

    conn.begin_test_transaction().unwrap();
    conn
}

#[cfg(feature = "mysql")]
pub fn establish_connection() -> diesel::MysqlConnection {
    use diesel::*;

    let mut conn = MysqlConnection::establish(
        &dotenv::var("DATABASE_URL")
            .or_else(|_| dotenv::var("MYSQL_DATABASE_URL"))
            .expect("Set either `DATABASE_URL` or `MYSQL_DATABASE_URL`"),
    )
    .unwrap();

    conn.begin_test_transaction().unwrap();

    conn
}
