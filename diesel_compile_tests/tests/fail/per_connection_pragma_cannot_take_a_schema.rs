extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel::sqlite::pragma::{PerConnection, Pragma, ReadPragma};

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

    let _ = connection.pragma::<ForeignKeys>(None);
    //~^ ERROR: type mismatch resolving `<ForeignKeys as Pragma>::Scope == PerSchema`
}
