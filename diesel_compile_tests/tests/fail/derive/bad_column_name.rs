#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
        #[sql_name = "spa ce"]
        space -> Text,
    }
}

#[derive(Queryable)]
#[diesel(table_name = users)]
struct User1 {
    #[diesel(column_name)]
    name: String,
}

#[derive(Queryable)]
#[diesel(table_name = users)]
struct User2 {
    #[diesel(column_name(another))]
    name: String,
}

#[derive(Queryable)]
#[diesel(table_name = users)]
struct User3 {
    #[diesel(column_name = true)]
    name: String,
}


#[derive(Insertable)]
#[diesel(table_name = users)]
struct User4 {
    #[diesel(column_name = "spa ce")]
    space: String,
}


#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct User5 {
    #[diesel(column_name = "spa ce")]
    space: String,
}

fn main() {}
