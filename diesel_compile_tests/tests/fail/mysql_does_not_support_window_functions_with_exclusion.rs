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
            dsl::count(users::id).frame_by(
                dsl::frame::Rows
                    .frame_start_with_exclusion(dsl::frame::CurrentRow, dsl::frame::ExcludeGroup),
            ),
        )
        .load::<i64>(&mut connection)
        .unwrap();

    let res =
        users::table
            .select(dsl::count(users::id).frame_by(
                dsl::frame::Rows.frame_start_with_exclusion(
                    dsl::frame::CurrentRow,
                    dsl::frame::ExcludeCurrentRow,
                ),
            ))
            .load::<i64>(&mut connection)
            .unwrap();

    let res = users::table
        .select(
            dsl::count(users::id).frame_by(
                dsl::frame::Rows
                    .frame_start_with_exclusion(dsl::frame::CurrentRow, dsl::frame::ExcludeTies),
            ),
        )
        .load::<i64>(&mut connection)
        .unwrap();

    let res =
        users::table
            .select(dsl::count(users::id).frame_by(
                dsl::frame::Rows.frame_start_with_exclusion(
                    dsl::frame::CurrentRow,
                    dsl::frame::ExcludeNoOthers,
                ),
            ))
            .load::<i64>(&mut connection)
            .unwrap();

    let res = users::table
        .select(Box::new(
            dsl::count(users::id).frame_by(
                dsl::frame::Rows
                    .frame_start_with_exclusion(dsl::frame::CurrentRow, dsl::frame::ExcludeGroup),
            ),
        ) as Box<dyn BoxableExpression<_, _, SqlType = _>>)
        .load::<i64>(&mut connection)
        .unwrap();
}
