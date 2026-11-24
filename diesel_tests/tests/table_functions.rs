use crate::schema::*;
use diesel::prelude::*;

#[cfg(feature = "postgres")]
mod pg {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::pg::Pg;
    use diesel::query_builder::AsQuery;
    use diesel::sql_types::{Bool, Integer, Text};

    diesel::view! {
        series {
            value -> Integer,
        }
    }

    diesel::view! {
        users_named {
            user_name -> Text,
            user_hair -> Nullable<Text>,
        }
    }

    diesel::allow_tables_to_appear_in_same_query!(users, users_named, series);

    #[diesel::declare_sql_function]
    extern "SQL" {
        #[query_source(series, base_type)]
        fn generate_series(start: Integer, stop: Integer) -> series::SqlType;

        #[sql_name = "pg_temp.users_named"]
        #[query_source(users_named)]
        fn users_named(pattern: Text) -> users_named::SqlType;
    }

    fn connection_with_users_named() -> TestConnection {
        let mut connection = connection_with_sean_and_tess_in_users_table();
        connection
            .batch_execute(
                "CREATE FUNCTION pg_temp.users_named(pattern text) \
                 RETURNS TABLE (user_name text, user_hair text) \
                 AS $$ SELECT name, hair_color FROM users WHERE name LIKE pattern $$ \
                 LANGUAGE sql STABLE",
            )
            .unwrap();
        connection
    }

    #[diesel_test_helper::test]
    fn a_call_renders_as_a_from_item_named_after_its_relation() {
        let query = users_named("S%").as_query();
        assert_eq!(
            diesel::debug_query::<Pg, _>(&query).to_string(),
            "SELECT \"users_named\".\"user_name\", \"users_named\".\"user_hair\" \
             FROM pg_temp.users_named($1) AS \"users_named\" -- binds: [\"S%\"]"
        );
    }

    #[diesel_test_helper::test]
    fn a_base_type_call_names_its_column_after_the_view() {
        let query = generate_series(1, 3).select(series::value);
        assert_eq!(
            diesel::debug_query::<Pg, _>(&query).to_string(),
            "SELECT \"series\".\"value\" FROM generate_series($1, $2) AS \"series\"(\"value\") \
             -- binds: [1, 3]"
        );

        let low = diesel::alias!(series as low);
        let query = generate_series(1, 3)
            .aliased(low)
            .select(low.field(series::value));
        assert_eq!(
            diesel::debug_query::<Pg, _>(&query).to_string(),
            "SELECT \"low\".\"value\" FROM generate_series($1, $2) AS \"low\"(\"value\") \
             -- binds: [1, 3]"
        );
    }

    #[diesel_test_helper::test]
    fn a_base_type_call_loads_its_rows() {
        let connection = &mut connection();
        let values = generate_series(1, 3)
            .select(series::value)
            .order(series::value.desc())
            .load::<i32>(connection)
            .unwrap();
        assert_eq!(values, [3, 2, 1]);
    }

    #[diesel_test_helper::test]
    fn a_table_returning_call_loads_its_rows() {
        let connection = &mut connection_with_users_named();
        let rows = users_named("%")
            .order(users_named::user_name)
            .load::<(String, Option<String>)>(connection)
            .unwrap();
        assert_eq!(
            rows,
            [("Sean".to_string(), None), ("Tess".to_string(), None)]
        );
    }

    #[diesel_test_helper::test]
    fn a_call_joins_a_table() {
        let connection = &mut connection_with_users_named();
        let rows = users::table
            .inner_join(users_named("S%").on(users_named::user_name.eq(users::name)))
            .select((users::name, users_named::user_name))
            .load::<(String, String)>(connection)
            .unwrap();
        assert_eq!(rows, [("Sean".to_string(), "Sean".to_string())]);

        let rows = users::table
            .left_join(users_named("S%").on(users_named::user_name.eq(users::name)))
            .select((users::name, users_named::user_name.nullable()))
            .order(users::name)
            .load::<(String, Option<String>)>(connection)
            .unwrap();
        assert_eq!(
            rows,
            [
                ("Sean".to_string(), Some("Sean".to_string())),
                ("Tess".to_string(), None)
            ]
        );
    }

    #[diesel_test_helper::test]
    fn two_aliased_calls_of_a_base_type_function_join() {
        let connection = &mut connection();
        let (low, high) = diesel::alias!(series as low, series as high);
        let values = generate_series(1, 2)
            .aliased(low)
            .inner_join(
                generate_series(2, 3)
                    .aliased(high)
                    .on(low.field(series::value).eq(high.field(series::value))),
            )
            .select((low.field(series::value), high.field(series::value)))
            .load::<(i32, i32)>(connection)
            .unwrap();
        assert_eq!(values, [(2, 2)]);
    }

    #[diesel_test_helper::test]
    fn two_aliased_calls_of_a_table_returning_function_join() {
        let connection = &mut connection_with_users_named();
        let (first, second) = diesel::alias!(users_named as first, users_named as second);
        let rows = users_named("S%")
            .aliased(first)
            .inner_join(
                users_named("T%")
                    .aliased(second)
                    .on(true.into_sql::<Bool>()),
            )
            .select((
                first.field(users_named::user_name),
                second.field(users_named::user_name),
            ))
            .load::<(String, String)>(connection)
            .unwrap();
        assert_eq!(rows, [("Sean".to_string(), "Tess".to_string())]);
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use diesel::sql_types::{Integer, Text};
    use diesel::sqlite::Sqlite;

    diesel::view! {
        json_each {
            key -> Integer,
            value -> Text,
        }
    }

    diesel::view! {
        series {
            value -> Integer,
        }
    }

    diesel::allow_tables_to_appear_in_same_query!(users, json_each);

    #[diesel::declare_sql_function]
    extern "SQL" {
        #[query_source(json_each)]
        fn json_each(json: Text) -> json_each::SqlType;

        #[query_source(series, base_type)]
        fn generate_series(start: Integer, stop: Integer) -> series::SqlType;
    }

    #[diesel_test_helper::test]
    fn a_call_renders_as_a_from_item_named_after_its_relation() {
        let query = json_each(r#"["a"]"#).select(json_each::value);
        assert_eq!(
            diesel::debug_query::<Sqlite, _>(&query).to_string(),
            "SELECT `json_each`.`value` FROM json_each(?) AS `json_each` \
             -- binds: [\"[\\\"a\\\"]\"]"
        );
    }

    #[diesel_test_helper::test]
    fn a_base_type_call_has_no_column_alias_list() {
        let query = generate_series(1, 3).select(series::value);
        assert_eq!(
            diesel::debug_query::<Sqlite, _>(&query).to_string(),
            "SELECT `series`.`value` FROM generate_series(?, ?) AS `series` -- binds: [1, 3]"
        );
    }

    // This test reads query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn a_call_loads_its_rows() {
        let connection = &mut connection();
        let rows = json_each(r#"["b","a"]"#)
            .order(json_each::key.desc())
            .load::<(i32, String)>(connection)
            .unwrap();
        assert_eq!(rows, [(1, "a".to_string()), (0, "b".to_string())]);
    }

    // This test reads query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn a_call_joins_a_table() {
        let connection = &mut connection_with_sean_and_tess_in_users_table();
        let rows = users::table
            .inner_join(json_each(r#"["Sean"]"#).on(json_each::value.eq(users::name)))
            .select((users::name, json_each::key))
            .load::<(String, i32)>(connection)
            .unwrap();
        assert_eq!(rows, [("Sean".to_string(), 0)]);

        let rows = users::table
            .left_join(json_each(r#"["Sean"]"#).on(json_each::value.eq(users::name)))
            .select((users::name, json_each::value.nullable()))
            .order(users::name)
            .load::<(String, Option<String>)>(connection)
            .unwrap();
        assert_eq!(
            rows,
            [
                ("Sean".to_string(), Some("Sean".to_string())),
                ("Tess".to_string(), None)
            ]
        );
    }

    // This test reads query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn two_aliased_calls_join() {
        let connection = &mut connection();
        let (first, second) = diesel::alias!(json_each as first, json_each as second);
        let rows = json_each(r#"["Sean","Tess"]"#)
            .aliased(first)
            .inner_join(
                json_each(r#"["Tess"]"#).aliased(second).on(first
                    .field(json_each::value)
                    .eq(second.field(json_each::value))),
            )
            .select((first.field(json_each::key), second.field(json_each::key)))
            .load::<(i32, i32)>(connection)
            .unwrap();
        assert_eq!(rows, [(1, 0)]);
    }

    // This test reads query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn a_call_groups_its_rows() {
        let connection = &mut connection();
        let rows = json_each(r#"["b","a","b"]"#)
            .group_by(json_each::value)
            .select((json_each::value, diesel::dsl::count_star()))
            .order(json_each::value)
            .load::<(String, i64)>(connection)
            .unwrap();
        assert_eq!(rows, [("a".to_string(), 1), ("b".to_string(), 2)]);
    }

    // This test reads query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn a_boxed_call_loads_its_rows() {
        let connection = &mut connection();
        let rows = json_each(r#"["a","b"]"#)
            .into_boxed()
            .filter(json_each::value.eq("b"))
            .load::<(i32, String)>(connection)
            .unwrap();
        assert_eq!(rows, [(1, "b".to_string())]);
    }

    // This test reads query results, which requires reading memory
    // allocated by the native sqlite library. Miri cannot track the
    // provenance of that memory when using `-Zmiri-native-lib`.
    #[cfg(not(miri))]
    #[diesel_test_helper::test]
    fn calls_combine_with_union() {
        let connection = &mut connection();
        let mut rows = json_each(r#"["a","a"]"#)
            .distinct()
            .select(json_each::value)
            .union(json_each(r#"["b"]"#).select(json_each::value))
            .load::<String>(connection)
            .unwrap();
        rows.sort();
        assert_eq!(rows, ["a", "b"]);

        let mut rows = json_each(r#"["a"]"#)
            .union_all(json_each(r#"["a"]"#))
            .load::<(i32, String)>(connection)
            .unwrap();
        rows.sort();
        assert_eq!(rows, [(0, "a".to_string()), (0, "a".to_string())]);
    }
}
