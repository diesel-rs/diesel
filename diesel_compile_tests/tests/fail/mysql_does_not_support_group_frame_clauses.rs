extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let mut connection = MysqlConnection::establish("").unwrap();

    let res = users::table
        .select(
            dsl::count(users::id)
                .window_order(users::name)
                .frame_by(dsl::frame::Groups.frame_start_with(dsl::frame::UnboundedPreceding)),
        )
        .load::<i64>(&mut connection)
        //~^ ERROR: `Groups` is no valid SQL fragment for the `Mysql` backend
        .unwrap();
}
