use schema::posts;

#[derive(Queryable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[insertable_into(posts)]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub body: &'a str,
}
