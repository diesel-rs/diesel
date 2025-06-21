extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();

    let res = users::table
        .select(
            dsl::count(users::id)
                .frame_by(dsl::frame::Groups.frame_start_with(dsl::frame::UnboundedPreceding)),
            //~^ ERROR: `Groups` frame clauses require a ordered window function
        )
        .load::<i64>(&mut connection)
        .unwrap();
}
