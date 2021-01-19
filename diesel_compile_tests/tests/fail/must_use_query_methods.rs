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
    st.select(b).distinct();
    st.count();
    st.order(b);
    st.limit(1);
    st.offset(1);

    st.filter(b.eq(true));
    st.filter(b.eq(true)).limit(1);

    insert_into(st);
    insert_into(st).values(&vec![b.eq(true), b.eq(false)]);

    update(st).set(b.eq(true));

    delete(st);

    let _thingies = st.filter(b.eq(true)); // No ERROR
}
