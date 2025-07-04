extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User {
    id: String,
    //~^ ERROR: the trait bound `std::string::String: FromSqlRow<diesel::sql_types::Integer, Pg>` is not satisfied
    name: i32,
    //~^ ERROR: the trait bound `i32: FromSqlRow<diesel::sql_types::Text, Pg>` is not satisfied
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct UserCorrect {
    id: i32,
    name: String,
}

#[derive(Selectable, Queryable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct SelectableWithEmbed {
    #[diesel(embed)]
    embed_user: User,
    //~^ ERROR: the trait bound `(String, i32): FromStaticSqlRow<(Integer, Text), Pg>` is not satisfied
}

fn main() {
    let mut conn = PgConnection::establish("...").unwrap();

    users::table
        .select(User::as_select())
        .load(&mut conn)
        //~^ ERROR: the trait bound `diesel::expression::select_by::SelectBy<User, _>: load_dsl::private::CompatibleType<_, _>` is not satisfied
        .unwrap();
    users::table
        .select(UserCorrect::as_select())
        .load(&mut conn)
        .unwrap();
}
