extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    posts {
        id -> Integer,
        title -> Text,
        user_id -> Integer,
    }
}

table! {
    comments {
        id -> Integer,
        text -> Text,
        post_id -> Integer,
    }
}

joinable!(comments -> posts (post_id));
joinable!(posts -> users (user_id));

allow_tables_to_appear_in_same_query!(users, posts, comments);
allow_columns_to_appear_in_same_group_by_clause!(
    posts::title,
    users::id,
    posts::user_id,
    users::name,
    posts::id,
    users::hair_color
);

fn main() {
    use diesel::dsl;
    // cases thas should compile

    // A column appering in the group by clause should be considered valid for the select clause
    let source = users::table.group_by(users::name).select(users::name);
    // If the column appearing in the group by clause is the primary key, any column of that table is a
    // valid group by clause
    let source = users::table.group_by(users::id).select(users::name);
    let source = users::table
        .group_by(users::id)
        .select((users::name, users::hair_color));
    // It's valid to use a aggregate function on a column that does not appear in the group by clause)
    let source = users::table
        .group_by(users::name)
        .select(dsl::max(users::id));
    // If the group by clause consists of multiple columns it's fine for the select clause to contain
    // any of those
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select(users::name);
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select(users::hair_color);
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select((users::name, users::hair_color));
    // It's fine to select all columns of a table as long as the primary key appears in the group by clause
    let source = users::table
        .inner_join(posts::table)
        .group_by(users::id)
        .select(users::all_columns);
    // This also work for group by clauses with multiple columns
    let source = users::table
        .inner_join(posts::table)
        .group_by((users::id, posts::title))
        .select((users::all_columns, posts::title));
    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .select((users::all_columns, posts::all_columns));

    // cases that should fail to compile
    let source = users::table.group_by(users::name).select(users::id);
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select(users::id);
    let source = users::table
        .group_by(users::name)
        .select((users::name, users::id));
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select(users::id);
    let source = users::table
        .inner_join(posts::table)
        .group_by((users::id, posts::title))
        .select((users::all_columns, posts::id));
    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .select((users::all_columns, posts::all_columns, comments::id));
}
