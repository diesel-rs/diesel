use diesel::*;
use super::User;

infer_schema!(dotenv!("DATABASE_URL"));

#[derive(PartialEq, Eq, Debug, Clone, Queryable)]
#[has_many(comments)]
#[belongs_to(user)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
    pub tags: Vec<String>,
}

impl Post {
    pub fn new(id: i32, user_id: i32, title: &str, body: Option<&str>) -> Self {
        Post {
            id: id,
            user_id: user_id,
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
            tags: Vec::new(),
        }
    }
}
