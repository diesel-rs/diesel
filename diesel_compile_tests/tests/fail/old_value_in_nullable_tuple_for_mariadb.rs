//@check-pass

// MariaDB 13 returns the old values for all of these, but they do not compile
// yet because a nullable tuple requires every element to be a `SqlType`

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

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
struct Previous {
    #[diesel(select_expression = old_value(users::name))]
    name: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
struct OptionallyEmbedded {
    #[diesel(embed)]
    previous: Option<Previous>,
    name: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
struct PreviousColor {
    #[diesel(select_expression = old_value(users::hair_color).assume_not_null())]
    hair_color: String,
}

// `previous` is `None` exactly when the old `hair_color` was NULL
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
struct Recolored {
    #[diesel(embed)]
    previous: Option<PreviousColor>,
    hair_color: Option<String>,
}

fn main() {
    let mut conn = MariadbConnection::establish("…").unwrap();

    let _: QueryResult<OptionallyEmbedded> = diesel::update(users::table.find(1))
        .set(users::name.eq("Renamed"))
        .returning(OptionallyEmbedded::as_select())
        .get_result(&mut conn);
    let _: QueryResult<Recolored> = diesel::update(users::table.find(1))
        .set(users::hair_color.eq("red"))
        .returning(Recolored::as_select())
        .get_result(&mut conn);
    let _: QueryResult<Option<(String, Option<String>)>> = diesel::update(users::table.find(1))
        .set(users::name.eq("Renamed"))
        .returning((old_value(users::name), old_value(users::hair_color)).nullable())
        .get_result(&mut conn);
}
