//@check-pass

extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {}

#[allow(dead_code)]
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
}

#[allow(dead_code)]
fn nullable_old_expressions_are_selectable_in_returning() {
    use pg::returning::old;

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(old(users::name).nullable());

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning((old(users::name).nullable(), users::name));

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq("Updated"))
        .returning(old(users::name).nullable());
}

#[allow(dead_code)]
fn nullable_boxed_expressions_are_selectable_in_returning() {
    use pg::Pg;
    use sql_types::Text;
    use std::boxed::Box;

    let boxed: Box<dyn BoxableExpression<users::table, Pg, SqlType = Text>> = Box::new(users::name);

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(boxed.nullable());
}

#[allow(dead_code)]
fn nullable_and_assume_not_null_expressions_are_selectable_in_update_returning() {
    use mariadb::returning::old_value;

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(old_value(users::name).nullable());

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning((old_value(users::name).nullable(), users::name));

    let _ = update(users::table)
        .set(users::name.eq("Updated"))
        .returning(users::name.assume_not_null());

    let _ = insert_into(users::table)
        .values(users::name.eq("Inserted"))
        .returning(users::name.assume_not_null());
}
