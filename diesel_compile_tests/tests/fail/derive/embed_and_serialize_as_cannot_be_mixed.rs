#[macro_use]
extern crate diesel;

table! {
    users (id) {
        id -> Integer,
        name -> Text,
        hair_color -> Text,
    }
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = users)]
struct NameAndHairColor<'a> {
    name: &'a str,
    hair_color: &'a str,
}

#[derive(Insertable)]
struct User<'a> {
    id: i32,
    #[diesel(embed, serialize_as = SomeType)]
    //~^ ERROR: `#[diesel(embed)]` cannot be combined with `#[diesel(serialize_as)]`
    // to test the compile error, this type doesn't need to exist
    name_and_hair_color: NameAndHairColor<'a>,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserChangeSet<'a> {
    id: i32,
    #[diesel(embed, serialize_as = SomeType)]
    //~^ ERROR: `#[diesel(embed)]` cannot be combined with `#[diesel(serialize_as)]`
    // to test the compile error, this type doesn't need to exist
    name_and_hair_color: NameAndHairColor<'a>,
}

fn main() {}
