use crate::helpers::connection;
use crate::schema;
use diesel::prelude::*;
use diesel_derives::Enum;

#[derive(Debug, Clone, Copy, Enum, PartialEq)]
#[cfg_attr(feature = "postgres", diesel(sql_type = schema::sql_types::Color))]
#[cfg_attr(feature = "mysql", diesel(sql_type = schema::sql_types::CarsPaintColorEnum))]
enum Color {
    Blue,
    Red,
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

#[test]
fn raw_sql_equal() {
    let v = Color::Red;
    let conn = &mut connection();

    #[cfg(feature = "postgres")]
    let r = diesel::select(diesel::dsl::sql::<schema::sql_types::Color>("'Red'::color").eq(v))
        .get_result::<bool>(conn)
        .unwrap();

    #[cfg(feature = "mysql")]
    let r =
        diesel::select(diesel::dsl::sql::<schema::sql_types::CarsPaintColorEnum>("'Red'").eq(v))
            .get_result::<bool>(conn)
            .unwrap();

    assert!(r);
}
