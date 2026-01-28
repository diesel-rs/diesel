extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(QueryableByName)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User {
    id: String,
    //~^ ERROR: the trait bound `std::string::String: FromSqlRow<diesel::sql_types::Integer, Pg>` is not satisfied
    name: i32,
    //~^ ERROR: the trait bound `i32: FromSqlRow<diesel::sql_types::Text, Pg>` is not satisfied
}

#[derive(QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User2 {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: String,
    //~^ ERROR: the trait bound `std::string::String: FromSqlRow<diesel::sql_types::Integer, Pg>` is not satisfied
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: i32,
    //~^ ERROR: the trait bound `i32: FromSqlRow<diesel::sql_types::Text, Pg>` is not satisfied
}

fn main() {
    let conn = &mut PgConnection::establish("…").unwrap();

    let s = diesel::sql_query("…").load::<User>(conn);
    //~^ ERROR: the trait bound `Untyped: load_dsl::private::CompatibleType<User, _>` is not satisfied
    //~| ERROR: the trait bound `User: FromSqlRow<_, _>` is not satisfied
}
