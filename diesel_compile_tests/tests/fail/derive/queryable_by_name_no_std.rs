//@check-pass
//@compile-flags: --crate-type lib
#![no_std]
#![allow(dead_code)]

mod core {}

#[derive(diesel::QueryableByName)]
struct Row {
    #[diesel(sql_type = diesel::sql_types::Integer, deserialize_as = i32)]
    value: Newtype,
}

struct Newtype(i32);

impl From<i32> for Newtype {
    fn from(value: i32) -> Self {
        Self(value)
    }
}
