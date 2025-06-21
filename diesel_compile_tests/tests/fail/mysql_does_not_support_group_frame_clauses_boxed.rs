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
        .select(Box::new(
            dsl::count(users::id).frame_by(
                dsl::frame::Rows
                    .frame_start_with_exclusion(dsl::frame::CurrentRow, dsl::frame::ExcludeGroup),
            ),
        ) as Box<dyn BoxableExpression<_, _, SqlType = _>>)
        //~^^^^^^ ERROR: `ExcludeGroup` is no valid SQL fragment for the `Mysql` backend
        .load::<i64>(&mut connection)
        .unwrap();
}
