extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    bad {
      id -> Integer,
      age -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, bad);

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    name: String,
}

fn main() {
    use self::users::dsl::*;

    let stmt = update(users.filter(id.eq(1)))
        .set(name.eq("Bill"))
        .returning(bad::age);
    //~^ ERROR: Cannot select `bad::columns::age` from `users::table`

    let new_user = NewUser {
        name: "Foobar".to_string(),
    };
    let stmt = insert_into(users)
        .values(&new_user)
        .returning((name, bad::age));
    //~^ ERROR: Cannot select `bad::columns::age` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
