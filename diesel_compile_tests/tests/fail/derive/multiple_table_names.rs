#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        group_id -> Integer,
    }
}

table! {
    users_ {
        id -> Integer,
        group_id -> Integer,
    }
}

table! {
    groups {
        id -> Integer,
    }
}

struct Group {
    id: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = users, table_name = users_)]
struct User1 {
    id: i32,
    group_id: i32,
}

#[derive(Associations)]
#[diesel(belongs_to(Group))]
#[diesel(table_name = users, table_name = users_)]
struct User2 {
    id: i32,
    group_id: i32,
}

#[derive(Identifiable)]
#[diesel(table_name = users, table_name = users_)]
struct User3 {
    id: i32,
    group_id: i32,
}

#[derive(Selectable)]
#[diesel(table_name = users)]
#[diesel(table_name = users_)]
struct User4 {
    id: i32,
    group_id: i32,
}

#[derive(Queryable)]
#[diesel(table_name = users, table_name = users_)]
struct User5 {
    id: i32,
    group_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(table_name = users_)]
struct User6 {
    id: i32,
    group_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users, table_name = users_)]
struct User7 {
    id: i32,
    group_id: i32,
}

fn main() {}
