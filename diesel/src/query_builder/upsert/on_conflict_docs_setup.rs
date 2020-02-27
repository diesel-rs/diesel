include!("../doctest_setup.rs");
use crate::schema::users;

#[derive(Clone, Copy, Insertable, AsChangeset)]
#[table_name="users"]
struct User<'a> {
    id: i32,
    name: &'a str,
}
