#[macro_use] extern crate diesel;

#[derive(Associations)]
//~^ ERROR Could not derive `Associations` without any `#[belongs_to(table_name)]` attribute
struct Post1 {
    a: i32,
}

#[derive(Associations)]
//~^ ERROR The foreign key column post_id is not found
#[belongs_to(Post)]
struct Post2 {
    a: i32,
}

#[derive(Associations)]
//~^ ERROR #[belongs_to]` must be in the form `#[belongs_to(table_name, foreign_key="column_name")]`
#[belongs_to(Post, abc="de")]
struct Post3 {
    a: i32,
    post_id: i32
}

#[derive(Associations)]
//~^ ERROR #[belongs_to]` must be in the form `#[belongs_to(table_name, foreign_key="column_name")]`
#[belongs_to(Post, foreign_key="post_id", abc="de")]
struct Post4 {
    a: i32,
    post_id: i32
}

fn main() {}
