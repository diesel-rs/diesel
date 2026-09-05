#[cfg(feature = "sqlite")]
use crate::schema::connection;
#[cfg(feature = "sqlite")]
use diesel::dsl::sql;
#[cfg(feature = "sqlite")]
use diesel::prelude::*;
#[cfg(feature = "sqlite")]
use diesel::sql_types::{Json, Jsonb};

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn json_arrow_operator_with_path() {
    let conn = &mut connection();

    let result = diesel::select(
        sql::<Json>(r#"json('{"a": {"b": [1, 2, 3]}}')"#).retrieve_as_object_sqlite("$.a.b[0]"),
    )
    .get_result::<serde_json::Value>(conn)
    .unwrap();
    assert_eq!(serde_json::json!(1), result);

    let result = diesel::select(
        sql::<Json>(r#"json('{"a": [1, 2, 3]}')"#).retrieve_as_object_sqlite("$.a[1]"),
    )
    .get_result::<serde_json::Value>(conn)
    .unwrap();
    assert_eq!(serde_json::json!(2), result);

    let result = diesel::select(
        sql::<Jsonb>(r#"json('{"a": {"b": [1, 2, 3]}}')"#).retrieve_as_object_sqlite("$.a.b[0]"),
    )
    .get_result::<serde_json::Value>(conn)
    .unwrap();
    assert_eq!(serde_json::json!(1), result);
}

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn json_arrow_operator_with_integer() {
    let conn = &mut connection();

    let result =
        diesel::select(sql::<Json>(r#"json('[10, 20, 30]')"#).retrieve_as_object_sqlite(0))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
    assert_eq!(serde_json::json!(10), result);

    let result =
        diesel::select(sql::<Json>(r#"json('[10, 20, 30]')"#).retrieve_as_object_sqlite(-1))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
    assert_eq!(serde_json::json!(30), result);
}

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn json_arrow_arrow_operator_with_path() {
    let conn = &mut connection();

    let result = diesel::select(sql::<Json>(r#"json('{"a": "xyz"}')"#).retrieve_as_text("$.a"))
        .get_result::<String>(conn)
        .unwrap();
    assert_eq!("xyz", result);

    let result = diesel::select(sql::<Json>(r#"json('{"a": 42}')"#).retrieve_as_text("$.a"))
        .get_result::<String>(conn)
        .unwrap();
    assert_eq!("42", result);

    let result = diesel::select(sql::<Jsonb>(r#"json('{"a": "xyz"}')"#).retrieve_as_text("$.a"))
        .get_result::<String>(conn)
        .unwrap();
    assert_eq!("xyz", result);
}

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn json_arrow_arrow_operator_missing_path_is_null() {
    let conn = &mut connection();

    let result = diesel::select(
        sql::<Json>(r#"json('{"a": 1}')"#)
            .retrieve_as_text("$.missing")
            .nullable(),
    )
    .get_result::<Option<String>>(conn)
    .unwrap();
    assert_eq!(None, result);
}

#[diesel_test_helper::test]
#[cfg(feature = "sqlite")]
fn json_arrow_arrow_operator_with_integer() {
    let conn = &mut connection();

    let result =
        diesel::select(sql::<Json>(r#"json('{"a": ["x", "y", "z"]}')"#).retrieve_as_text("$.a[1]"))
            .get_result::<String>(conn)
            .unwrap();
    assert_eq!("y", result);
}
