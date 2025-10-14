use crate::helpers::*;
use crate::schema::*;
use diesel_derives::Enum;

#[derive(Debug, Enum, PartialEq)]
#[diesel(backend(diesel::pg::Pg))]
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

#[test]
fn insert_color() {
    #[derive(Insertable)]
    struct Car {
        paint_color: Color,
    }

    let conn = &mut connection();
    let new_car = Car {
        paint_color: Color::Blue,
    };
    diesel::insert_into(cars::table)
        .values(new_car)
        .execute(conn)
        .unwrap();
}
