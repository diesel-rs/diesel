extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

fn main() {
    let mut connection = MysqlConnection::establish("").unwrap();
    users::table
        .full_join(posts::table.on(users::id.eq(posts::id)))
        .get_result::<(Option<i32>, Option<i32>)>(&mut connection);
}
