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
    let mut connection = MysqlConnection::establish("").unwrap();

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .returning(users::id)
        .get_result::<i32>(&mut connection);
    //~^ ERROR: `ReturningClause<id>` is no valid SQL fragment for the `Mysql` backend
}
