extern crate diesel;

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::mariadb::Mariadb;
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

#[derive(Debug, FromSqlRow)]
struct Nickname(String);

impl FromSql<diesel::sql_types::Text, Mariadb> for Nickname {
    fn from_sql(value: <Mariadb as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        <String as FromSql<diesel::sql_types::Text, Mariadb>>::from_sql(value).map(Nickname)
    }
}

fn main() {
    let mut conn = MariadbConnection::establish("…").unwrap();

    // any of those are accepted
    let _: (
        String,
        Option<String>,
        i32,
        Option<String>,
        Name,
        Nickname,
        Option<String>,
        String,
    ) = diesel::update(users::table.find(1))
        .set(users::name.eq("Renamed"))
        .returning((
            old_value(users::name),
            old_value(users::name).nullable(),
            old_value(users::id),
            old_value(users::hair_color),
            old_value(users::name),
            old_value(users::name),
            old_value(users::hair_color).nullable(),
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
        //~| ERROR: the trait bound `OldValueOf<Text>: SqlType` is not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::hair_color).is_null())
        //~^ ERROR: the method `is_null` exists for struct `OldValue<hair_color>`, but its trait bounds were not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::name).eq(old_value(users::name)))
        //~^ ERROR: the method `eq` exists for struct `diesel::mariadb::returning::old_impl::OldValue<columns::name>`, but its trait bounds were not satisfied
        .execute(&mut conn);

    // MariaDB evaluates this to the new value
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::name).concat("!"))
        //~^ ERROR: the method `concat` exists for struct `diesel::mariadb::returning::old_impl::OldValue<columns::name>`, but its trait bounds were not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(diesel::dsl::case_when(users::id.eq(1), old_value(users::name)))
        //~^ ERROR: the trait bound `OldValue<name>: AsExpression<OldValueOf<Text>>` is not satisfied
        //~| ERROR: the trait bound `OldValue<name>: AsExpression<OldValueOf<Text>>` is not satisfied
        //~| ERROR: the trait bound `OldValueOf<Nullable<Text>>: SqlType` is not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(old_value(users::id).cast::<diesel::sql_types::BigInt>())
        //~^ ERROR: the method `cast` exists for struct `diesel::mariadb::returning::old_impl::OldValue<columns::id>`, but its trait bounds were not satisfied
        .execute(&mut conn);

    // A nullable tuple of `old_value` is still no operand
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(diesel::dsl::case_when(
            //~^ ERROR: the trait bound `Nullable<(OldValueOf<Text>,)>: IntoNullable` is not satisfied
            users::id.eq(1),
            (old_value(users::name),).nullable(),
            //~^ ERROR: the trait bound `Nullable<(OldValue<name>,)>: AsExpression<...>` is not satisfied
        ))
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning((old_value(users::name),).nullable().assume_not_null())
        //~^ ERROR: cannot select `AssumeNotNull<Nullable<(OldValue<name>,)>>` from `ReturningQuerySource<UpdateStmt, table>`
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning((old_value(users::name),).nullable().eq((old_value(users::name),).nullable()))
        //~^ ERROR: the method `eq` exists for struct `Nullable<(OldValue<name>,)>`, but its trait bounds were not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning((old_value(users::name), old_value(users::id)).nullable().is_null())
        //~^ ERROR: the method `is_null` exists for struct `Nullable<(OldValue<name>, OldValue<id>)>`, but its trait bounds were not satisfied
        .execute(&mut conn);
    diesel::update(users::table)
        .set(users::name.eq("Renamed"))
        .returning(users::id.nullable().eq_any((old_value(users::id),).nullable()))
        //~^ ERROR: `Nullable<(OldValue<id>,)>` is not an iterator
        .execute(&mut conn);
}
