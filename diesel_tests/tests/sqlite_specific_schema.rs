use diesel::*;
use super::User;

//FIXME: We can go back to `infer_schema!` when codegen becomes generic
table! {
    users {
        id -> Integer,
        name -> VarChar,
        hair_color -> Nullable<VarChar>,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        title -> VarChar,
        body -> Nullable<Text>,
    }
}

table! {
    comments {
        id -> Integer,
        post_id -> Integer,
        text -> Text,
    }
}

table! {
    special_posts {
        id -> Integer,
        user_id -> Integer,
        title -> VarChar,
    }
}

table! {
    special_comments {
        id -> Integer,
        special_post_id -> Integer,
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable)]
#[has_many(comments)]
#[belongs_to(user)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

impl Post {
    pub fn new(id: i32, user_id: i32, title: &str, body: Option<&str>) -> Self {
        Post {
            id: id,
            user_id: user_id,
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
        }
    }
}
