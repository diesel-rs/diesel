extern crate diesel;

use diesel::prelude::*;
use diesel::sqlite::pragma::{PerSchema, Pragma, SchemaPragma, WritePragma};

struct UserVersion;

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl WritePragma for UserVersion {
    type Argument<'a> = i32;
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    let _ = UserVersion::set(7, &mut connection);
    //~^ ERROR: this function takes 3 arguments but 2 arguments were supplied
}
