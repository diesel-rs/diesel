extern crate diesel;

use diesel::dsl::*;
use diesel::*;

table! {
    foo (id) {
        id -> Int8,
    }
}

fn main() {
    foo::table.select(foo::id).only();
    //~^ ERROR: the method `only` exists for struct `SelectStatement<FromClause<table>, SelectClause<id>>`, but its trait bounds were not satisfied
    foo::table.select(foo::id).filter(foo::id.eq(1)).only();
    //~^ ERROR: the method `only` exists for struct `SelectStatement<FromClause<table>, SelectClause<id>, ..., ...>`, but its trait bounds were not satisfied
    foo::table.select(foo::id.only());
    //~^ ERROR: the method `only` exists for struct `columns::id`, but its trait bounds were not satisfied

    // .only() is not supported for SQLite
    let mut conn = SqliteConnection::establish("").unwrap();
    foo::table.only().load(&mut conn).unwrap();
    //~^ ERROR: the trait bound `Only<foo::table>: LoadQuery<'_, diesel::SqliteConnection, _>` is not satisfied

    // .only() is not supported for MySql
    let mut conn = MysqlConnection::establish("").unwrap();
    foo::table.only().load(&mut conn).unwrap();
    //~^ ERROR: the trait bound `Only<foo::table>: LoadQuery<'_, diesel::MysqlConnection, _>` is not satisfied
}
