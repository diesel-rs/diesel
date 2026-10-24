extern crate diesel;

use diesel::prelude::*;
use diesel::sqlite::pragma::{PerSchema, Pragma, PragmaLiteral, SchemaPragma, WritePragma};

struct UserVersion(i32);

impl Pragma for UserVersion {
    const NAME: &'static str = "user_version";
    type Scope = PerSchema;
}

impl WritePragma for UserVersion {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    let _ = UserVersion(7).set(&mut connection);
    //~^ ERROR: this method takes 2 arguments but 1 argument was supplied
}
