extern crate diesel;

use diesel::pg::PgConnection;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    posts (user_id) {
        user_id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
    }
}

fn main() {
    use self::posts::dsl::*;
    use self::users::dsl::*;
    let mut conn = PgConnection::establish("").unwrap();

    // Sanity check, valid query
    insert_into(posts).values(users).execute(&mut conn).unwrap();

    insert_into(posts).values(vec![users, users]);
    //~^ ERROR: the trait bound `users::table: UndecoratedInsertRecord<posts::table>` is not satisfied

    insert_into(posts).values((users, users));
    //~^ ERROR: type mismatch resolving `<table as Insertable<table>>::Values == ValuesClause<_, table>`
    //~| ERROR: type mismatch resolving `<table as Insertable<table>>::Values == ValuesClause<_, table>`
}
