include!("src/doctest_setup.rs");

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Clone, Copy, Insertable)]
#[table_name="users"]
struct User<'a> {
    id: i32,
    name: &'a str,
}
