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

table! {
    non_users {
        id -> Integer,
        noname -> VarChar,
    }
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();

    delete(users::table.filter(users::columns::name.eq("Bill")))
        .returning(non_users::columns::noname);

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .returning(non_users::columns::noname);

    update(users::table)
        .set(users::columns::name.eq("Bill"))
        .returning(non_users::columns::noname);
}
