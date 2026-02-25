use crate::helpers::*;
use crate::schema::*;
use diesel::prelude::*;
use diesel_derives::Enum;

#[derive(Debug, Enum, PartialEq)]
#[diesel(check_for_backend(diesel::pg::Pg), sql_type = sql_types::Color)]
enum Color {
    Blue,
    Red,
}

#[test]
fn as_bytes() {
    let expected = b"Blue";
    let actual = Color::Blue.as_bytes();

    assert_eq!(expected, actual);
}

#[test]
fn from_bytes() {
    assert_eq!(Color::from_bytes(b"Red").unwrap(), Color::Red);
}
