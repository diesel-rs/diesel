use diesel::dsl;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    let mut conn = MysqlConnection::establish("…").unwrap();

    // no support for aggregate filters
    users::table
        .select(dsl::count(users::id).aggregate_filter(users::name.eq("Sean")))
        .get_result::<i64>(&mut conn)
        //~^ ERROR: `Filter<WhereClause<Grouped<Eq<name, Bound<Text, &str>>>>>` is no valid SQL fragment for the `Mysql` backend
        .unwrap();

    // no support for aggregate order
    users::table
        .select(dsl::count(users::id).aggregate_order(users::name))
        .get_result::<i64>(&mut conn)
        //~^ ERROR: `Order<name, false>` is no valid SQL fragment for the `Mysql` backend
        .unwrap();
}
