use diesel::prelude::*;

#[derive(Queryable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Queryable)]
pub struct Company {
    pub company_id: i32,
    pub company_code: String,
    pub company_name: String,    
}