extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        title -> Text,
    }
}

joinable!(posts -> users (user_id));
allow_tables_to_appear_in_same_query!(users, posts);

#[derive(Queryable, Selectable)]
#[diesel(table_name = posts)]
struct Post {
    id: i32,
    title: String,
}

fn title_of(post: Post) -> (i32, String) {
    (post.id, post.title)
}

fn main() {
    // `Option::<Post>::as_select()` selects an optional `Selectable`
    let query = users::table
        .left_join(posts::table)
        .select((users::name, (Post::as_select(),).nullable()));
    let _ = diesel::debug_query::<diesel::sqlite::Sqlite, _>(&query).to_string();
    let mut conn = SqliteConnection::establish("…").unwrap();
    let _ = query.get_result::<(String, Option<Post>)>(&mut conn).unwrap();
    //~^ ERROR: the trait bound `(Text, Nullable<(...,)>): CompatibleType<..., _>` is not satisfied
}
