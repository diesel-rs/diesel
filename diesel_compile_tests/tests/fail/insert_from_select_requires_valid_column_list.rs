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

table! {
    comments (post_id) {
        post_id -> Integer,
        body -> Nullable<Text>,
    }
}

fn main() {
    use self::posts::dsl::*;
    use self::users::dsl::*;
    let mut conn = PgConnection::establish("").unwrap();

    // Sanity check, valid query with no column list
    users.insert_into(posts).execute(&mut conn).unwrap();

    // Sanity check, valid query with single column
    users
        .select(id)
        .insert_into(posts)
        .into_columns(user_id)
        .execute(&mut conn)
        .unwrap();

    // Sanity check, valid query with column list
    users
        .select((name, hair_color))
        .insert_into(posts)
        .into_columns((title, body))
        .execute(&mut conn)
        .unwrap();

    // No column list, mismatched types
    users
        .select((name, hair_color))
        .insert_into(posts)
        .execute(&mut conn)
        //~^ ERROR: type mismatch resolving `<SelectStatement<..., ...> as Query>::SqlType == (..., ..., ...)`
        .unwrap();

    // Single column, wrong table
    users
        .select(id)
        .insert_into(posts)
        .into_columns(comments::post_id);
    //~^ ERROR: type mismatch resolving `<post_id as ColumnList>::Table == table`

    // Single column, wrong type
    users.select(id).insert_into(posts).into_columns(title);
    //~^ ERROR: type mismatch resolving `<title as Expression>::SqlType == Integer`

    // Multiple columns, one from wrong table
    users
        .select((id, name))
        .insert_into(posts)
        .into_columns((comments::post_id, title));
    //~^ ERROR: the trait bound `(post_id, title): ColumnList` is not satisfied

    // Multiple columns, both from wrong table
    users
        .select((id, hair_color))
        .insert_into(posts)
        .into_columns((comments::post_id, comments::body));
    //~^ ERROR: type mismatch resolving `<post_id as ColumnList>::Table == table`
    //~| ERROR: type mismatch resolving `<body as ColumnList>::Table == table`

    // Multiple columns, one wrong type
    users
        .select((id, name))
        .insert_into(posts)
        .into_columns((user_id, body));
    //~^ ERROR: type mismatch resolving `<(user_id, body) as Expression>::SqlType == (Integer, Text)`

    // Multiple columns, both wrong types
    users
        .select((id, name))
        .insert_into(posts)
        .into_columns((title, body));
    //~^ ERROR: type mismatch resolving `<(title, body) as Expression>::SqlType == (Integer, Text)`
}
