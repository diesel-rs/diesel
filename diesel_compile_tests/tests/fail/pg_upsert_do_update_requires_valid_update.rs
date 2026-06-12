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

allow_tables_to_appear_in_same_query!(users, posts);

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
    //~^ ERROR: the method `execute` exists for struct `IncompleteDoUpdate<InsertStatement<..., ...>, ...>`, but its trait bounds were not satisfied

    // Update column from other table
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(posts::title.eq("Sean"));
    //~^ ERROR: type mismatch resolving `<Grouped<...> as AsChangeset>::Target == table`

    // Update column with value that is not selectable
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`

    // Update column with excluded value that is not selectable
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq(excluded(posts::title)));
    //~^ ERROR: type mismatch resolving `<title as Column>::Table == table`

    // Update column with excluded value of wrong type
    insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_update()
        .set(name.eq(excluded(id)));
    //~^ ERROR: the trait bound `Excluded<id>: AsExpression<Text>` is not satisfied

    // Excluded is only valid in upsert
    // FIXME: This should not compile
    update(users)
        .set(name.eq(excluded(name)))
        .execute(&mut connection);
}
