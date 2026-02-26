use crate::schema;
use diesel_derives::Enum;
use crate::helpers::connection;
use diesel::prelude::*;

#[derive(Debug, Enum, PartialEq)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "mysql", diesel(check_for_backend(diesel::mysql::Mysql)))]
#[diesel(sql_type = schema::sql_types::Color)]
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
    #[derive(Debug, PartialEq, Insertable, HasQuery)]
    #[diesel(table_name = schema::cars)]
    struct Car {
        id: i32,
        paint_color: Color
    }

    let conn = &mut connection();
    let new_car = Car {
        id: 0,
        paint_color: Color::Blue
    };
    diesel::insert_into(schema::cars::table)
        .values(new_car)
        .execute(conn)
        .unwrap();

    let saved = Car::query().load(conn).unwrap();
    let expected = vec![new_car];
    assert_eq!(expected, saved);
}
