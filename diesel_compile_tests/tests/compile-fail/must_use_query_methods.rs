#![deny(unused_must_use)]

#[macro_use]
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

    st.select(b); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.select(b).distinct(); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.count(); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.order(b); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.limit(1); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.offset(1); //~ ERROR unused `diesel::query_builder::SelectStatement`

    st.filter(b.eq(true)); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.filter(b.eq(true)).limit(1); //~ ERROR unused `diesel::query_builder::SelectStatement`

    insert_into(st); //~ ERROR unused
    insert_into(st).values(&vec![b.eq(true), b.eq(false)]); //~ ERROR unused

    update(st).set(b.eq(true)); //~ ERROR unused

    delete(st); //~ ERROR unused

    let _thingies = st.filter(b.eq(true)); // No ERROR
}
