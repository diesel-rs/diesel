#[macro_use]
extern crate diesel;

#[derive(Queryable)]
#[diesel(what = true)]
struct User1 {
    id: i32,
}

#[derive(Queryable)]
struct User2 {
    #[diesel(what = true)]
    id: i32,
}

fn main() {}
