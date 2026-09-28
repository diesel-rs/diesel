extern crate diesel;

use diesel::mariadb::returning::old_value;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

#[derive(Debug, diesel::types::Enum)]
#[diesel(sql_type = diesel::sql_types::Text)]
enum Name {
    Sean,
    Tess,
}

fn main() {
    let mut conn = MariadbConnection::establish("…").unwrap();

    // any of those are accepted
    let _: (String, Option<String>, i32, Option<String>, Name, String) =
        diesel::update(users::table.find(1))
            .set(users::name.eq("Renamed"))
            .returning((
                old_value(users::name),
                old_value(users::name).nullable(),
                old_value(users::id),
                old_value(users::hair_color),
                old_value(users::name),
                users::name,
            ))
            .get_result(&mut conn)
            .unwrap();

    // MariaDB rejects these as syntax errors
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::name).eq("Sean"))
        //~^ ERROR: the method `eq` exists for struct `diesel::mariadb::returning::old_impl::OldValue<columns::name>`, but its trait bounds were not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::id) + 1)
        //~^ ERROR: cannot add `{integer}` to `diesel::mariadb::returning::old_impl::OldValue<columns::id>`
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(users::name.eq(old_value(users::name)))
        //~^ ERROR: the trait bound `OldValue<name>: AsExpression<Text>` is not satisfied
        //~| ERROR: the trait bound `OldValueOf<diesel::sql_types::Text>: diesel::sql_types::SqlType` is not satisfied
        .execute(&mut conn);

    // MariaDB evaluates this to the new value
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::name).concat("!"))
        //~^ ERROR: the method `concat` exists for struct `diesel::mariadb::returning::old_impl::OldValue<columns::name>`, but its trait bounds were not satisfied
        .execute(&mut conn);
}
