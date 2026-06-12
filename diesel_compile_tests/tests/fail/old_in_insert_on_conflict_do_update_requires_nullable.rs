extern crate diesel;

use diesel::pg::returning::old;
use diesel::prelude::*;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

// Mirrors the supported `Selectable` shape, but without wrapping `old(name)`
// in `.nullable()`. In an `INSERT ... ON CONFLICT ... DO UPDATE`, freshly
// inserted rows have no `old` row, so `old.col` would be `NULL` for them;
// loading into a non-nullable `String` would therefore be unsound. Diesel
// rejects this at compile time.
#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
struct UpsertOldNew {
    #[diesel(select_expression = old(users::name))]
    was: String,
    name: String,
}

fn main() {
    let mut connection = PgConnection::establish("…").unwrap();

    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq(""))
        .returning(UpsertOldNew::as_select())
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .get_result::<UpsertOldNew>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();

    // The plain tuple version mirrors the same constraint: writing
    // `old(name)` without `.nullable()` is rejected.
    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq(""))
        .returning(old(users::name))
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .get_result::<String>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();

    // With Nullable, this compiles
    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq(""))
        .returning(old(users::name).nullable())
        .get_result::<Option<String>>(&mut connection)
        .unwrap();
}
