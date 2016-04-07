#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen)]

#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[insertable_into(users)]
struct NewUser<'a> {
    name: &'a str,
}

fn main() {
    use self::users::dsl::*;

    let connection = PgConnection::establish("").unwrap();
    let new_user = NewUser { name: "Robbie" };
    let stmt = users.select(name);
    insert(&new_user).into(users).returning(name).union(stmt);
    //~^ ERROR no method named `union` found for type `diesel::query_builder::insert_statement::InsertQuery<users::columns::name, diesel::query_builder::insert_statement::InsertStatement<users::table, &NewUser<'_>, diesel::query_builder::insert_statement::Insert>>` in the current scope

    update(users.find(1)).set(name.eq("Robert")).returning(name).union(stmt);
    //~^ ERROR no method named `union` found for type `diesel::query_builder::update_statement::UpdateQuery<users::columns::name, diesel::query_builder::update_statement::UpdateStatement<diesel::query_source::filter::FilteredQuerySource<users::table, diesel::expression::predicates::Eq<users::columns::id, diesel::expression::bound::Bound<diesel::types::Integer, i32>>>, diesel::expression::predicates::Eq<users::columns::name, diesel::expression::bound::Bound<diesel::types::Text, &str>>>>` in the current scope
}
