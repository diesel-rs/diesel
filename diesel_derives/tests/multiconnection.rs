use crate::schema::users;
use diesel::prelude::*;

#[derive(diesel::MultiConnection)]
pub enum InferConnection {
    #[cfg(feature = "postgres")]
    Pg(PgConnection),
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConnection),
    #[cfg(feature = "mysql")]
    Mysql(MysqlConnection),
}

#[derive(Queryable, Selectable)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[test]
fn check_queries_work() {
    let database_url = if cfg!(feature = "mysql") {
        dotenvy::var("MYSQL_UNIT_TEST_DATABASE_URL").or_else(|_| dotenvy::var("DATABASE_URL"))
    } else if cfg!(feature = "postgres") {
        dotenvy::var("PG_DATABASE_URL").or_else(|_| dotenvy::var("DATABASE_URL"))
    } else {
        Ok(dotenvy::var("DATABASE_URL").unwrap_or_else(|_| ":memory:".to_owned()))
    };
    let database_url = database_url.expect("DATABASE_URL must be set in order to run tests");

    let mut conn = InferConnection::establish(&database_url).unwrap();

    diesel::sql_query(
        "CREATE TEMPORARY TABLE users(\
         id INTEGER NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL)",
    )
    .execute(&mut conn)
    .unwrap();

    conn.begin_test_transaction().unwrap();

    // these are mostly compile pass tests

    // simple query
    let _ = users::table
        .select((users::id, users::name))
        .load::<User>(&mut conn)
        .unwrap();

    // with bind
    let _ = users::table
        .select((users::id, users::name))
        .find(42)
        .load::<User>(&mut conn)
        .unwrap();

    // simple boxed query
    let _ = users::table
        .into_boxed()
        .select((users::id, users::name))
        .load::<User>(&mut conn)
        .unwrap();

    // with bind
    let _ = users::table
        .into_boxed()
        .select((users::id, users::name))
        .filter(users::id.eq(42))
        .load::<User>(&mut conn)
        .unwrap();

    // as_select
    let _ = users::table
        .select(User::as_select())
        .load(&mut conn)
        .unwrap();

    // boxable expression
    let b = Box::new(users::name.eq("John"))
        as Box<
            dyn BoxableExpression<
                users::table,
                self::multi_connection_impl::MultiBackend,
                SqlType = _,
            >,
        >;

    let _ = users::table
        .filter(b)
        .select(users::id)
        .load::<i32>(&mut conn)
        .unwrap();

    // insert
    diesel::insert_into(users::table)
        .values((users::id.eq(42), users::name.eq("John")))
        .execute(&mut conn)
        .unwrap();
    // update
    diesel::update(users::table)
        .set(users::name.eq("John"))
        .execute(&mut conn)
        .unwrap();

    // delete
    diesel::delete(users::table).execute(&mut conn).unwrap();
}
