extern crate diesel;

use diesel::pg::PgConnection;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Text,
    }
}

fn main() {
    use self::users::dsl::*;
    let mut conn = PgConnection::establish("").unwrap();

    insert_into(users).values(&name.eq(1));
    //~^ ERROR: the trait bound `{integer}: AsExpression<diesel::sql_types::Text>` is not satisfied
}
