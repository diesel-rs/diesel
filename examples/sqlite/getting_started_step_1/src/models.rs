use diesel::prelude::*;

// Order matters here!
// Queryable assumes that the order of field on the struct matches the columns in the corresponding table.
// See https://docs.diesel.rs/diesel/deserialize/trait.Queryable.html#derivingfor details
#[derive(Queryable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}
