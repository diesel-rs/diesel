extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Binary,
    }
}

fn main() {
    use self::users::dsl::*;

    let mut connection = SqliteConnection::establish("").unwrap();

    users
        .select(name.concat(b"foo".to_vec()))
        .filter(name.like(b"bar".to_vec()))
        .filter(name.not_like(b"baz".to_vec()))
        .get_result::<Vec<u8>>(&mut connection).unwrap();

    let mut connection = MysqlConnection::establish("").unwrap();

    users
        .select(name.concat(b"foo".to_vec()))
        .filter(name.like(b"bar".to_vec()))
        .filter(name.not_like(b"baz".to_vec()))
        .get_result::<Vec<u8>>(&mut connection).unwrap();
}
