extern crate diesel;

use diesel::*;

table!{
    users{
       id -> Integer,
       name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
    }
}

fn main() {
    let mut conn = PgConnection::establish("").unwrap();
    let subquery = users::table.filter(users::id.eq(1));
    let query = posts::table.filter(posts::user_id.eq_any(subquery));
}
