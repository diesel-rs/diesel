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
    name: i32,
}

#[derive(QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User2 {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: i32,
}

fn main() {
    let conn = &mut PgConnection::establish("…").unwrap();

    let s = diesel::sql_query("…").load::<User>(conn);
}
