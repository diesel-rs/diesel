extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let mut connection = MysqlConnection::establish("…").unwrap();

    // sanity checks
    // no errors
    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict(dsl::DuplicatedKeys)
        .do_nothing()
        .execute(&mut connection);

    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict_do_nothing()
        .execute(&mut connection);

    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict(dsl::DuplicatedKeys)
        .do_update()
        .set(name.eq("Jane"))
        .execute(&mut connection);

    // do not allow columns as on_conflict target
    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict(name)
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: OnConflictValues<ValuesClause<..., ...>, ..., ...>` is no valid SQL fragment for the `Mysql` backend

    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict((id, name))
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: `OnConflictValues<ValuesClause<..., ...>, ..., ...>` is no valid SQL fragment for the `Mysql` backend

    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict((dsl::DuplicatedKeys, name))
        //~^ ERROR: the trait bound `ConflictTarget<(..., ...)>: OnConflictTarget<...>` is not satisfied
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: `OnConflictValues<ValuesClause<..., ...>, ..., ...>` is no valid SQL fragment for the `Mysql` backend

    // do not allow raw sql fragments as on_conflict target
    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict(dsl::sql("foo"))
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: `OnConflictValues<ValuesClause<..., ...>, ..., ...>` is no valid SQL fragment for the `Mysql` backend

    // do not allow excluded
    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict(dsl::DuplicatedKeys)
        .do_update()
        .set(name.eq(upsert::excluded(name)))
        .execute(&mut connection);
    //~^ ERROR: `OnConflictValues<ValuesClause<..., ...>, ..., ...>` is no valid SQL fragment for the `Mysql` backend

    let mut connection = PgConnection::establish("postgres://localhost").unwrap();

    // do not allow `DuplicatedKeys` for other backends:
    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict(dsl::DuplicatedKeys)
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: `ConflictTarget<DuplicatedKeys>` is no valid SQL fragment for the `Pg` backend

    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict((name, dsl::DuplicatedKeys))
        //~^ ERROR: the trait bound `ConflictTarget<(name, ...)>: OnConflictTarget<...>` is not satisfied
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: `ConflictTarget<(columns::name, DuplicatedKeys)>` is no valid SQL fragment for the `Pg` backend

    insert_into(users)
        .values((id.eq(42), name.eq("John")))
        .on_conflict((dsl::DuplicatedKeys, name))
        //~^ ERROR: the trait bound `ConflictTarget<(..., ...)>: OnConflictTarget<...>` is not satisfied
        .do_nothing()
        .execute(&mut connection);
    //~^ ERROR: `ConflictTarget<(DuplicatedKeys, columns::name)>` is no valid SQL fragment for the `Pg` backend
}
