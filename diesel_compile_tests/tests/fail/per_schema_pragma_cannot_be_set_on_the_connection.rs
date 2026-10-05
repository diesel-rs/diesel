extern crate diesel;

use diesel::prelude::*;
use diesel::sqlite::pragma::{PerSchema, Pragma, WritePragma};

struct UserVersion;

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl WritePragma for UserVersion {
    type Value = i32;
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    let _ = connection.set_connection_pragma::<UserVersion>(&7);
    //~^ ERROR: type mismatch resolving `<UserVersion as Pragma>::Scope == PerConnection`
}
