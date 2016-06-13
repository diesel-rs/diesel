#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    int_primary_key {
        id -> Integer,
    }
}

table! {
    string_primary_key {
        id -> VarChar,
    }
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    int_primary_key::table.find("1").first(&connection).unwrap();
    //~^ ERROR no method named `first` found for type `diesel::query_source::filter::FilteredQuerySource<int_primary_key::table, diesel::expression::predicates::Eq<int_primary_key::columns::id, &str>>` in the current scope
    //~| ERROR E0277
    string_primary_key::table.find(1).first(&connection).unwrap();
    //~^ ERROR no method named `first` found for type `diesel::query_source::filter::FilteredQuerySource<string_primary_key::table, diesel::expression::predicates::Eq<string_primary_key::columns::id, _>>` in the current scope
    //~| ERROR E0277
}
