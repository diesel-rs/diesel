extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

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
    use self::users::dsl::*;
    use self::posts::dsl::*;
    let mut conn = PgConnection::establish("").unwrap();

    // Sanity check, valid query with no column list
    users
        .insert_into(posts)
        .execute(&mut conn)
        .unwrap();

    // Sanity check, valid query with single column
    users.select(id)
        .insert_into(posts)
        .into_columns(user_id)
        .execute(&mut conn)
        .unwrap();

    // Sanity check, valid query with column list
    users.select((name, hair_color))
        .insert_into(posts)
        .into_columns((title, body))
        .execute(&mut conn)
        .unwrap();

    // No column list, mismatched types
    users.select((name, hair_color))
        .insert_into(posts)
        .execute(&mut conn)
        .unwrap();

    // Single column, wrong table
    users.select(id)
        .insert_into(posts)
        .into_columns(comments::post_id);

    // Single column, wrong type
    users.select(id)
        .insert_into(posts)
        .into_columns(title);

    // Multiple columns, one from wrong table
    users.select((id, name))
        .insert_into(posts)
        .into_columns((comments::post_id, title));

    // Multiple columns, both from wrong table
    users.select((id, hair_color))
        .insert_into(posts)
        .into_columns((comments::post_id, comments::body));

    // Multiple columns, one wrong type
    users.select((id, name))
        .insert_into(posts)
        .into_columns((user_id, body));

    // Multiple columns, both wrong types
    users.select((id, name))
        .insert_into(posts)
        .into_columns((title, body));
}
