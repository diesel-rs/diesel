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
    use stuff::table as st;
    use stuff::b;

    st.select(b); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.select(b).distinct(); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.count(); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.order(b); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.limit(1); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.offset(1); //~ ERROR unused `diesel::query_builder::SelectStatement`

    st.filter(b.eq(true)); //~ ERROR unused `diesel::query_builder::SelectStatement`
    st.filter(b.eq(true)).limit(1); //~ ERROR unused `diesel::query_builder::SelectStatement`

    insert_into(stuff)
        .values(b.eq(true)); //- ERROR unused `diesel::query_builder::InsertStatement`
    insert_into(stuff)
        .values(&vec![b.eq(true), b.eq(false)]); //~ ERROR unused `diesel::query_builder::InsertStatement`

    update(stuff).set(b.eq(true)); //~ ERROR unused `diesel::query_builder::UpdateStatement`

    delete(stuff); //~ ERROR unused `diesel::query_builder::DeleteStatement`

    let _thingies = st.filter(b.eq(true)); // No ERROR
}
