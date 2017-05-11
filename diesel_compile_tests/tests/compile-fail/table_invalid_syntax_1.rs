#[macro_use] extern crate diesel;

table! {
    12
    //~^ ERROR expected ident
}

fn main() {}
