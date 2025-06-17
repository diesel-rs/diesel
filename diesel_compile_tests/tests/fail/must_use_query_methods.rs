#![deny(unused_must_use)]

extern crate diesel;

use diesel::*;

table! {
    stuff (b) {
        b -> Bool,
    }
}

fn main() {
    use stuff::b;
    use stuff::table as st;

    st.select(b);
    //~^ ERROR: unused `SelectStatement` that must be used
    st.select(b).distinct();
    //~^ ERROR: unused `SelectStatement` that must be used
    st.count();
    //~^ ERROR: unused `SelectStatement` that must be used
    st.order(b);
    //~^ ERROR: unused `SelectStatement` that must be used
    st.limit(1);
    //~^ ERROR: unused `SelectStatement` that must be used
    st.offset(1);
    //~^ ERROR: unused `SelectStatement` that must be used

    st.filter(b.eq(true));
    //~^ ERROR: unused `SelectStatement` that must be used
    st.filter(b.eq(true)).limit(1);
    //~^ ERROR: unused `SelectStatement` that must be used

    insert_into(st);
    //~^ ERROR: unused `IncompleteInsertStatement` that must be used
    insert_into(st).values(&vec![b.eq(true), b.eq(false)]);
    //~^ ERROR: unused `InsertStatement` that must be used

    update(st).set(b.eq(true));
    //~^ ERROR: unused `UpdateStatement` that must be used

    delete(st);
    //~^ ERROR: unused `DeleteStatement` that must be used

    let _thingies = st.filter(b.eq(true)); // No ERROR
}
