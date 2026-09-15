#[cfg(feature = "postgres")]
pub fn create_user_table(conn: &mut diesel::PgConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users (id Serial PRIMARY KEY, name TEXT NOT NULL DEFAULT '', hair_color TEXT)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "postgres")]
pub fn create_posts_table(conn: &mut diesel::PgConnection) {
    use diesel::*;

    diesel::sql_query(
        "CREATE TEMPORARY TABLE posts (id Serial PRIMARY KEY, user_id INTEGER NOT NULL)",
    )
    .execute(conn)
    .unwrap();
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
pub fn create_user_table(conn: &mut diesel::SqliteConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL DEFAULT '', hair_color TEXT)")
        .execute(conn)
        .unwrap();
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
pub fn create_posts_table(conn: &mut diesel::SqliteConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "mysql")]
pub fn create_user_table(conn: &mut diesel::MysqlConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TEMPORARY TABLE users (id INTEGER PRIMARY KEY AUTO_INCREMENT, name TEXT NOT NULL, hair_color TEXT)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "mysql")]
pub fn create_posts_table(conn: &mut diesel::MysqlConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TEMPORARY TABLE posts (id INTEGER PRIMARY KEY AUTO_INCREMENT, user_id INTEGER NOT NULL)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "mariadb")]
pub fn create_user_table(conn: &mut diesel::MariadbConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TEMPORARY TABLE users (id INTEGER PRIMARY KEY AUTO_INCREMENT, name TEXT NOT NULL, hair_color TEXT)")
        .execute(conn)
        .unwrap();
}

#[cfg(feature = "mariadb")]
pub fn create_posts_table(conn: &mut diesel::MariadbConnection) {
    use diesel::*;

    diesel::sql_query("CREATE TEMPORARY TABLE posts (id INTEGER PRIMARY KEY AUTO_INCREMENT, user_id INTEGER NOT NULL)")
        .execute(conn)
        .unwrap();
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
pub fn establish_connection() -> diesel::SqliteConnection {
    use diesel::*;

    SqliteConnection::establish(":memory:").unwrap()
}

#[cfg(feature = "postgres")]
pub fn establish_connection() -> diesel::PgConnection {
    use diesel::*;

    let mut conn = PgConnection::establish(
        &dotenvy::var("DATABASE_URL")
            .or_else(|_| dotenvy::var("PG_DATABASE_URL"))
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
        &dotenvy::var("DATABASE_URL")
            .or_else(|_| dotenvy::var("MYSQL_DATABASE_URL"))
            .expect("Set either `DATABASE_URL` or `MYSQL_DATABASE_URL`"),
    )
    .unwrap();

    conn.begin_test_transaction().unwrap();

    conn
}

#[cfg(feature = "mariadb")]
pub fn establish_connection() -> diesel::MariadbConnection {
    use diesel::*;

    let mut conn = MariadbConnection::establish(
        &dotenvy::var("DATABASE_URL")
            .or_else(|_| dotenvy::var("MARIADB_DATABASE_URL"))
            .expect("Set either `DATABASE_URL` or `MARIADB_DATABASE_URL`"),
    )
    .unwrap();

    conn.begin_test_transaction().unwrap();

    conn
}
