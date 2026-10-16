extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel::sqlite::pragma::{ConnectionPragma, PerConnection, Pragma, ReadPragma};

struct ForeignKeys;

impl Pragma for ForeignKeys {
    const NAME: &'static str = "foreign_keys";
    type Scope = PerConnection;
}

impl ReadPragma for ForeignKeys {
    type SqlType = Bool;
    type Row = bool;
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    let _ = ForeignKeys::get(&mut connection, None);
    //~^ ERROR: this function takes 1 argument but 2 arguments were supplied
}
