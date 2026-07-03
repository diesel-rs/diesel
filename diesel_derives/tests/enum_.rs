use crate::helpers::TestConnection;
use crate::helpers::connection;
use crate::schema;
use diesel::deserialize::FromSql;
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::query_builder::{Query, QueryFragment, QueryId};
use diesel::serialize::ToSql;
use diesel_derives::Enum;

#[derive(Debug, Clone, Copy, Enum, PartialEq)]
#[cfg_attr(feature = "postgres", diesel(sql_type = schema::sql_types::Color))]
#[cfg_attr(feature = "mysql", diesel(sql_type = schema::sql_types::CarsPaintColorEnum))]
#[diesel(sql_type = diesel::sql_types::Text)]
#[diesel(sql_type = diesel::sql_types::Integer)]
enum Color {
    Blue = 0,
    Red = 1,
}

#[test]
#[cfg(any(feature = "postgres", feature = "mysql"))]
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
#[cfg(any(feature = "postgres", feature = "mysql"))]
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

#[test]
fn raw_sql_equal_int() {
    let v = Color::Red;
    let conn = &mut connection();

    let r = diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>().eq(v))
        .get_result::<bool>(conn)
        .unwrap();

    assert!(r);
}

#[test]
fn raw_sql_equal_text() {
    let v = Color::Red;
    let conn = &mut connection();

    let r = diesel::select("Red".into_sql::<diesel::sql_types::Text>().eq(v))
        .get_result::<bool>(conn)
        .unwrap();

    assert!(r);
}

#[test]
fn deserialize_int() {
    let v = Color::Red;
    let conn = &mut connection();

    let r = diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>())
        .get_result::<Color>(conn)
        .unwrap();

    assert_eq!(r, v);
}

#[test]
fn deserialize_string() {
    let v = Color::Red;
    let conn = &mut connection();

    let r = diesel::select("Red".into_sql::<diesel::sql_types::Text>())
        .get_result::<Color>(conn)
        .unwrap();

    assert_eq!(r, v);
}

#[track_caller]
fn check_variant<'a, V>(conn: &mut TestConnection, v: V, expected: &'a str)
where
    V: AsExpression<diesel::sql_types::Text>
        + Copy
        + PartialEq
        + FromSql<diesel::sql_types::Text, <TestConnection as Connection>::Backend>
        + ToSql<diesel::sql_types::Text, <TestConnection as Connection>::Backend>
        + diesel::Queryable<diesel::sql_types::Text, <TestConnection as Connection>::Backend>
        + 'static,
    V::Expression: Expression<SqlType = diesel::sql_types::Text>,
    diesel::dsl::select<V::Expression>: Query<SqlType = diesel::sql_types::Text>
        + QueryFragment<<TestConnection as Connection>::Backend>
        + QueryId,
    diesel::dsl::select<diesel::dsl::IntoSql<&'a str, diesel::sql_types::Text>>: Query<SqlType = diesel::sql_types::Text>
        + QueryFragment<<TestConnection as Connection>::Backend>
        + QueryId,
{
    let r = diesel::select(v.into_sql::<diesel::sql_types::Text>())
        .get_result::<String>(conn)
        .unwrap();
    assert_eq!(r, expected);

    let r = diesel::select(expected.into_sql::<diesel::sql_types::Text>())
        .get_result::<V>(conn)
        .unwrap();
    assert_eq!(r, v);
}

#[test]
fn rename() {
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "lowercase")]
    enum LowerCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "UPPERCASE")]
    enum UpperCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "PascalCase")]
    enum PascalCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "camelCase")]
    enum CamelCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "snake_case")]
    enum SnakeCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "SCREAMING_SNAKE_CASE")]
    enum ScreamingSnakeCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "kebab-case")]
    enum KebabCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #[diesel(rename_all = "SCREAMING-KEBAB-CASE")]
    enum ScreamingKebabCase {
        TestName,
    }
    #[derive(PartialEq, Enum, Debug, Copy, Clone)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    enum Custom {
        #[diesel(rename = "custom")]
        TestName,
    }

    let conn = &mut connection();
    check_variant(conn, LowerCase::TestName, "testname");
    check_variant(conn, UpperCase::TestName, "TESTNAME");
    check_variant(conn, PascalCase::TestName, "TestName");
    check_variant(conn, CamelCase::TestName, "testName");
    check_variant(conn, SnakeCase::TestName, "test_name");
    check_variant(conn, ScreamingSnakeCase::TestName, "TEST_NAME");
    check_variant(conn, KebabCase::TestName, "test-name");
    check_variant(conn, ScreamingKebabCase::TestName, "TEST-NAME");
    check_variant(conn, Custom::TestName, "custom");
}

#[test]
fn check_max_color() {
    #[derive(Debug, Enum, PartialEq, Clone, Copy)]
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    #[repr(i64)]
    enum Color {
        Blue = -10_000_000,
        Red = 200_000_000_000,
    }

    let conn = &mut connection();

    let v = Color::Blue;
    let expected = v as i64;

    let r = diesel::select(v.into_sql::<diesel::sql_types::BigInt>())
        .get_result::<i64>(conn)
        .unwrap();
    assert_eq!(r, expected);

    let r = diesel::select(expected.into_sql::<diesel::sql_types::BigInt>())
        .get_result::<Color>(conn)
        .unwrap();
    assert_eq!(r, v);
}
