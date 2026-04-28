use crate::helpers::connection;
use crate::schema;
use diesel::prelude::*;
use diesel_derives::Enum;

#[derive(Debug, Clone, Copy, Enum, PartialEq)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg), sql_type = schema::sql_types::Color))]
#[cfg_attr(feature = "mysql", diesel(check_for_backend(diesel::mysql::Mysql), sql_type = schema::sql_types::CarsPaintColorEnum))]
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
fn insert_and_select() {
    #[derive(Debug, Clone, Copy, PartialEq, Insertable, HasQuery)]
    #[diesel(table_name = schema::cars)]
    struct Car {
        id: i32,
        paint_color: Color,
    }

    let conn = &mut connection();
    let new_car = Car {
        id: 1,
        paint_color: Color::Blue,
    };
    diesel::insert_into(schema::cars::table)
        .values(new_car)
        .execute(conn)
        .unwrap();

    let saved = Car::query().load(conn).unwrap();
    let expected = vec![new_car];
    assert_eq!(expected, saved);
}
