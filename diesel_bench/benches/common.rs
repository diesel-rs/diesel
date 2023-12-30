#[cfg(all(feature = "sqlx", feature = "sqlite"))]
type Id = i64;
#[cfg(not(all(feature = "sqlx", feature = "sqlite")))]
type Id = i32;

diesel::table! {
    comments {
        id -> Integer,
        post_id -> Integer,
        text -> Text,
    }
}

diesel::table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
    }
}

diesel::table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

diesel::joinable!(comments -> posts (post_id));
diesel::joinable!(posts -> users (user_id));
diesel::allow_tables_to_appear_in_same_query!(users, posts, comments);

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    diesel::Associations,
    diesel::Identifiable,
    diesel::Queryable,
)]
#[diesel(belongs_to(Post))]
pub struct Comment {
    pub id: Id,
    pub post_id: Id,
    pub text: String,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    diesel::Associations,
    diesel::Identifiable,
    diesel::Queryable,
    diesel::QueryableByName,
)]
#[diesel(belongs_to(User))]
#[diesel(table_name = posts)]
pub struct Post {
    pub id: Id,
    pub user_id: Id,
    pub title: String,
    pub body: Option<String>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    diesel::AsChangeset,
    diesel::Identifiable,
    diesel::Insertable,
    diesel::Queryable,
    diesel::QueryableByName,
)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub hair_color: Option<String>,
}
