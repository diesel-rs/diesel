//@check-pass

extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    nullable_expressions_are_selectable_in_returning();
    nullable_old_expressions_are_selectable_in_returning();
    nullable_boxed_expressions_are_selectable_in_returning();
    nullable_and_assume_not_null_expressions_are_selectable_in_update_returning();
}

fn nullable_expressions_are_selectable_in_returning() {
    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(users::name.nullable());

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .returning(users::name.nullable());

    let _ = delete(users::table).returning(users::name.nullable());

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .on_conflict(users::id)
        .do_nothing()
        .returning(users::name.nullable());

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq("Updated"))
        .returning(users::name.nullable());

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning((users::id, users::name.nullable()));

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(users::id + 1);

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(users::id + users::id);
}

fn nullable_old_expressions_are_selectable_in_returning() {
    use pg::returning::old;

    let mut conn = PgConnection::establish("…").unwrap();

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(old(users::name).nullable())
        .execute(&mut conn);

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning((old(users::name).nullable(), users::name))
        .execute(&mut conn);

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq("Updated"))
        .returning(old(users::name).nullable())
        .execute(&mut conn);
}

fn nullable_boxed_expressions_are_selectable_in_returning() {
    use pg::Pg;
    use sql_types::Text;

    let mut conn = PgConnection::establish("…").unwrap();

    let boxed: Box<dyn BoxableExpression<users::table, Pg, SqlType = Text>> = Box::new(users::name);

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(boxed.nullable())
        .execute(&mut conn);
}

fn nullable_and_assume_not_null_expressions_are_selectable_in_update_returning() {
    use mariadb::returning::old_value;

    let mut conn = MariadbConnection::establish("…").unwrap();

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(old_value(users::name).nullable())
        .execute(&mut conn);

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning((old_value(users::name).nullable(), users::name))
        .execute(&mut conn);

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(users::name.assume_not_null())
        .execute(&mut conn);

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .returning(users::name.assume_not_null())
        .execute(&mut conn);
}
