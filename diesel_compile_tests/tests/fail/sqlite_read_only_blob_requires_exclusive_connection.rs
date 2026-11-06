//@no-rustfix
use diesel::prelude::*;

table! {
    files (id) {
        id -> BigInt,
        data -> Binary,
    }
}

fn main() {
    let connection = SqliteConnection::establish("").unwrap();
    // A hook can reach its connection through a shared reference, so opening a blob must not be possible through one.
    let shared: &SqliteConnection = &connection;
    let _ = shared.get_read_only_blob(files::data, 1);
    //~^ ERROR: cannot borrow `*shared` as mutable, as it is behind a `&` reference
}
