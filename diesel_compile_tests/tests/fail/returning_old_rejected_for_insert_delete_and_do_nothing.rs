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

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser(#[diesel(column_name = name)] String);

fn main() {
    use self::users::dsl::*;

    let mut connection = PgConnection::establish("").unwrap();

    // Plain INSERT: `old(col)` is meaningless because no pre-existing row exists.
    insert_into(users)
        .values(&NewUser("Hello".into()))
        .returning(old(name))
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .get_result::<String>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();

    // DELETE: `old(col)` is pointless because all columns already refer to the
    // row being deleted — `RETURNING col` suffices.
    delete(users.filter(id.eq(1)))
        .returning(old(name))
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<DeleteStmt, table>`
        .get_result::<String>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<DeleteStmt, table>`
        .unwrap();

    // INSERT ... ON CONFLICT DO NOTHING: conflicting rows are never returned,
    // so `old(col)` would always be NULL and is misleading.
    insert_into(users)
        .values(&NewUser("Hello".into()))
        .on_conflict(id)
        .do_nothing()
        .returning(old(name))
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .get_result::<String>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `ReturningQuerySource<..., ...>`
        .unwrap();
}
