extern crate diesel;
use diesel::prelude::*;
use diesel::r2d2::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value = false)]
struct NewUser {
    name: &'static str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
struct User {
    name: String,
}

fn main() {
    let conn = &mut PgConnection::establish("_").unwrap();

    // that works
    diesel::copy_from(users::table)
        .from_insertable(vec![NewUser { name: "John" }])
        .execute(conn)
        .unwrap();
    diesel::copy_to(users::table).load::<User, _>(conn).unwrap();

    let manager = ConnectionManager::<PgConnection>::new("");
    let pool = Pool::builder().max_size(2).build(manager).unwrap();
    let mut conn = pool.get().unwrap();

    diesel::copy_from(users::table)
        .from_insertable(vec![NewUser { name: "John" }])
        .execute(&mut conn)
        .unwrap();
    diesel::copy_to(users::table)
        .load::<User, _>(&mut conn)
        .unwrap();

    let conn = &mut MysqlConnection::establish("_").unwrap();

    // that fails
    diesel::copy_from(users::table)
        .from_insertable(vec![NewUser { name: "John" }])
        .execute(conn)
        //~^ ERROR: type mismatch resolving `<MysqlConnection as Connection>::Backend == Pg`
        //~| ERROR: the trait bound `CopyFromQuery<table, InsertableWrapper<...>>: ExecuteCopyFromDsl<...>` is not satisfied
        .unwrap();
    diesel::copy_to(users::table).load::<User, _>(conn).unwrap();
    //~^ ERROR: the trait bound `diesel::MysqlConnection: pg::query_builder::copy::copy_to::ExecuteCopyToConnection` is not satisfied
    //~| ERROR: the trait bound `diesel::MysqlConnection: pg::query_builder::copy::copy_to::ExecuteCopyToConnection` is not satisfied

    let conn = &mut SqliteConnection::establish("_").unwrap();

    // that fails
    diesel::copy_from(users::table)
        .from_insertable(vec![NewUser { name: "John" }])
        .execute(conn)
        //~^ ERROR: type mismatch resolving `<SqliteConnection as Connection>::Backend == Pg`
        //~| ERROR: the trait bound `CopyFromQuery<table, InsertableWrapper<...>>: ExecuteCopyFromDsl<...>` is not satisfied
        .unwrap();
    diesel::copy_to(users::table).load::<User, _>(conn).unwrap();
    //~^ ERROR: the trait bound `diesel::SqliteConnection: pg::query_builder::copy::copy_to::ExecuteCopyToConnection` is not satisfied
    //~| ERROR: the trait bound `diesel::SqliteConnection: pg::query_builder::copy::copy_to::ExecuteCopyToConnection` is not satisfied
}
