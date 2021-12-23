extern crate diesel;

use diesel::upsert::*;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        title -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser(#[diesel(column_name = name)] &'static str);

#[allow(deprecated)]
fn main() {
    use self::users::dsl::*;
    let mut connection = PgConnection::establish("postgres://localhost").unwrap();

    // Valid update as sanity check
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq("Sean"))
        .execute(&mut connection);

    // No set clause
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .execute(&mut connection);

    // Update column from other table
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(posts::title.eq("Sean"));

    // Update column with value that is not selectable
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq(posts::title));

    // Update column with excluded value that is not selectable
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq(excluded(posts::title)));

    // Update column with excluded value of wrong type
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq(excluded(id)));

    // Excluded is only valid in upsert
    // FIXME: This should not compile
    update(users).set(name.eq(excluded(name))).execute(&mut connection);
}
