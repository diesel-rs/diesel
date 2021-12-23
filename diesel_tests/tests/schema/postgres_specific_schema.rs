use super::{posts, User};

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations, QueryableByName)]
#[diesel(table_name = posts)]
#[diesel(belongs_to(User))]
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
