extern crate diesel;
use diesel::prelude::*;

diesel::table! {
    users(id) {
        id -> Integer,
        name -> Nullable<Text>,
    }
}

#[derive(Selectable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: Option<String>,
    #[diesel(
        select_expression = users::name.is_null(),
        select_expression_type = diesel::dsl::IsNull<users::name>
    )]
    name_is_null: bool,
    #[diesel(
        select_expression = (users::name, users::id),
        select_expression_type = (users::name, users::id)
    )]
    name_and_id: (Option<String>, i32),
    non_existing: String,
    #[diesel(
        select_expression = users::non_existing,
        select_expression_type = users::non_existing
    )]
    non_existing_with_annotation: String,
    #[diesel(
        select_expression = (users::id, users::non_existing),
        select_expression_type = (users::id, users::non_existing)
    )]
    non_existing_in_tuple: (i32, String),
    #[diesel(
        select_expression = (users::id + 45),
        select_expression_type = users::id,
    )]
    no_tuple: i32,
}

#[derive(Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User1<'a> {
    name: &'a str,
}

fn main() {}
