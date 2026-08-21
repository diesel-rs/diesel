use diesel::prelude::*;

table! {
    users {
        #[auto_increment]
        id -> Integer,
        name -> Text,
    }
}

table! {
    plain_items {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    let mut conn = MysqlConnection::establish("…").unwrap();

    // a batch insert generates several ids, only one could be reported
    diesel::insert_into(users::table)
        .values(vec![users::name.eq("Sean"), users::name.eq("Tess")])
        .execute_returning_id(&mut conn)
        //~^ ERROR: may insert zero or several rows
        .unwrap();

    // a table without `#[auto_increment]` sets no generated id
    diesel::insert_into(plain_items::table)
        .values(plain_items::name.eq("Sean"))
        .execute_returning_id(&mut conn)
        //~^ ERROR: has no column marked `#[auto_increment]`
        .unwrap();

    // `INSERT ... SELECT` may insert zero or several rows
    users::table
        .select(users::name)
        .insert_into(users::table)
        .into_columns(users::name)
        .execute_returning_id(&mut conn)
        //~^ ERROR: may insert zero or several rows
        .unwrap();
}
