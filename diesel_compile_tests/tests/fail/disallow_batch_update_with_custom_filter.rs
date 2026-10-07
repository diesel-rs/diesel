extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(AsChangeset, Identifiable)]
#[diesel(table_name = users)]
struct Changeset {
    id: i32,
    name: String,
}

fn main() {
    let conn = &mut SqliteConnection::establish("…").unwrap();

    let changes = [Changeset {
        id: 1,
        name: "John".into(),
    }];

    // sanity check that everything is supposed to work
    let _ = diesel::update(users::table).set(&changes).execute(conn);

    // we don't allow filter or find in any position
    let _ = diesel::update(users::table.find(42))
        .set(&changes)
        .execute(conn);
    //~^ ERROR: cannot apply a `WHERE` clause to batch updates
    let _ = diesel::update(users::table.filter(users::id.eq(42)))
        .set(&changes)
        .execute(conn);
    //~^ ERROR: cannot apply a `WHERE` clause to batch updates
    let _ = diesel::update(users::table)
        .filter(users::id.eq(42))
        .set(&changes)
        .execute(conn);
    //~^ ERROR: cannot apply a `WHERE` clause to batch updates

    let _ = diesel::update(users::table)
        .set(&changes)
        .filter(users::id.eq(42))
        .execute(conn);
    //~^ ERROR: cannot apply a `WHERE` clause to batch updates
}
