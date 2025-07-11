extern crate diesel;

use diesel::*;

table! {
    users{
       id -> Integer,
       name -> Nullable<Text>,
    }
}

fn main() {
    let mut conn = PgConnection::establish("").unwrap();
    // Should not be allowed because `users::name` is nullable, so the result of `eq_any` is
    // nullable as well.
    let _: Vec<bool> = users::table
        .select(users::name.eq_any(["foo", "bar"]))
        .load(&mut conn)
        //~^ ERROR: cannot deserialize a value of the database type `diesel::sql_types::Nullable<Bool>` as `bool`
        .unwrap();
}
