#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let connection = Connection::establish("").unwrap();
    let select_id = users.select(id);
    let select_name = users.select(name);

    let ids: Vec<i32> = select_name.load(&connection).unwrap().collect();
    //~^ ERROR the trait `diesel::query_source::Queryable<diesel::types::VarChar>` is not implemented for the type `i32`
    //~| ERROR E0277
    let names: Vec<String> = select_id.load(&connection).unwrap().collect();
    //~^ ERROR the trait `diesel::query_source::Queryable<diesel::types::Integer>` is not implemented
    //~| ERROR E0277
}
