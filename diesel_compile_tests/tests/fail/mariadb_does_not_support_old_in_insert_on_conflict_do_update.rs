extern crate diesel;

use diesel::mariadb::returning::old_value;
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
    #[diesel(select_expression = old_value(users::name))]
    was: String,
    name: String,
}

fn main() {
    let mut connection = MariadbConnection::establish("…").unwrap();

    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(diesel::dsl::DuplicatedKeys)
        .do_update()
        .set(users::name.eq(""))
        .returning(UpsertOldNew::as_select())
        //~^ ERROR: cannot select `diesel::mariadb::returning::old_impl::OldValue<columns::name>` from `ReturningQuerySource<..., ...>`
        .get_result::<UpsertOldNew>(&mut connection)
        //~^ ERROR: cannot select `diesel::mariadb::returning::old_impl::OldValue<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();

    // The plain tuple version mirrors the same constraint: writing
    // `old_value(name)` without is rejected.
    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(diesel::dsl::DuplicatedKeys)
        .do_update()
        .set(users::name.eq(""))
        .returning(old_value(users::name))
        //~^ ERROR: cannot select `diesel::mariadb::returning::old_impl::OldValue<columns::name>` from `ReturningQuerySource<..., ...>`
        .get_result::<String>(&mut connection)
        //~^ ERROR: cannot select `diesel::mariadb::returning::old_impl::OldValue<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();

    // Even With Nullable this does not compile
    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(diesel::dsl::DuplicatedKeys)
        .do_update()
        .set(users::name.eq(""))
        .returning(old_value(users::name).nullable())
        //~^ ERROR: cannot select `diesel::mariadb::returning::old_impl::OldValue<columns::name>` from `ReturningQuerySource<..., ...>`
        //~| ERROR: the trait bound `ReturningQuerySource<..., ...>: Table` is not satisfied
        .get_result::<Option<String>>(&mut connection)
        //~^ ERROR: cannot select `diesel::mariadb::returning::old_impl::OldValue<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();

    // Sanity check: returning the column itself works
    insert_into(users::table)
        .values(users::name.eq(""))
        .on_conflict(diesel::dsl::DuplicatedKeys)
        .do_update()
        .set(users::name.eq(""))
        .returning(users::name)
        .get_result::<String>(&mut connection)
        .unwrap();

    // On Update it compiles
    update(users::table)
        .set(users::name.eq(""))
        .returning(old_value(users::name))
        .get_result::<String>(&mut connection)
        .unwrap();
}
