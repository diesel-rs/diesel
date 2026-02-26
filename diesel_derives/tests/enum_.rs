use crate::schema::sql_types::Color;
use diesel_derives::Enum;

#[derive(Debug, Enum, PartialEq)]
#[diesel(check_for_backend(diesel::pg::Pg), sql_type = Color)]
enum ColorEnum {
    Blue,
    Red,
}

#[test]
fn as_bytes() {
    let expected = b"Blue";
    let actual = ColorEnum::Blue.as_bytes();

    assert_eq!(expected, actual);
}

#[test]
fn from_bytes() {
    assert_eq!(ColorEnum::from_bytes(b"Red").unwrap(), ColorEnum::Red);
}
