//@no-rustfix
use diesel::prelude::*;
use std::pin::Pin;

fn test<T: Unpin>() {}

fn main() {
    test::<SqliteConnection>();
    //~^ ERROR: `PhantomPinned` cannot be unpinned
}

fn first_test() {
    let mut conn = SqliteConnection::establish("…").unwrap();
    let mut pinned = std::pin::pin!(conn);

    let p = pinned.get_mut();
    //~^ ERROR: `PhantomPinned` cannot be unpinned
}

fn other() {
    let mut conn = SqliteConnection::establish("…").unwrap();
    let mut pinned = std::pin::pin!(conn);

    std::mem::replace(&mut *pinned, SqliteConnection::establish("…").unwrap());
    //~^ ERROR: cannot borrow data in dereference of `Pin<&mut diesel::SqliteConnection>` as mutable
}
