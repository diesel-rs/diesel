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

    st.select(b); //~ ERROR unused result
    st.select(b).distinct(); //~ ERROR unused result
    st.count(); //~ ERROR unused result
    st.order(b); //~ ERROR unused result
    st.limit(1); //~ ERROR unused result
    st.offset(1); //~ ERROR unused result

    st.filter(b.eq(true)); //~ ERROR unused result
    st.filter(b.eq(true)).limit(1); //~ ERROR unused result

    let _thingies = st.filter(b.eq(true)); // No ERROR
}
