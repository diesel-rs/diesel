extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser(#[diesel(column_name = name)] String);

fn main() {
    use self::users::dsl::*;

    let mut connection = PgConnection::establish("").unwrap();

    let query = delete(users.filter(name.eq("Bill"))).returning(id);
    query.returning(name);

    let query = insert_into(users)
        .values(&NewUser("Hello".into()))
        .returning(id);
    query.returning(name);

    let query = update(users).set(name.eq("Bill")).returning(id);
    query.returning(name);
}
