#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;
    let connection = SqliteConnection::establish(":memory:").unwrap();

    update(users.filter(id.eq(1)))
        .set(name.eq("Bill"))
        .get_result(&connection);
    //~^ ERROR SupportsReturningClause

    update(users.filter(id.eq(1)))
        .set(name.eq("Bill"))
        .returning(name)
        .get_result(&connection);
    //~^ ERROR SupportsReturningClause
}
