extern crate diesel;

use diesel::dsl::count;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    name: String,
}

fn main() {
    use self::users::dsl::*;

    let stmt = update(users.filter(id.eq(1)))
        .set(name.eq("Bill"))
        .returning(count(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    let new_user = NewUser {
        name: "Foobar".to_string(),
    };
    let stmt = insert_into(users)
        .values(&new_user)
        .returning((name, count(name)));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL
}
