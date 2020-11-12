use diesel::sql_types::Integer;
use diesel::*;

use helpers::connection;

table! {
    my_structs (foo) {
        foo -> Integer,
        bar -> Integer,
    }
}

#[test]
fn named_struct_definition() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = my_structs)]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = &mut connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(conn);
    assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
}

#[test]
fn tuple_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = my_structs)]
    struct MyStruct(
        #[diesel(column_name = foo)] i32,
        #[diesel(column_name = bar)] i32,
    );

    let conn = &mut connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(conn);
    assert_eq!(Ok(MyStruct(1, 2)), data);
}

#[test]
fn struct_with_path_in_name() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = self::my_structs)]
    struct MyStruct {
        foo: i32,
        bar: i32,
    }

    let conn = &mut connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(conn);
    assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
}

// FIXME: Test usage with renamed columns

#[test]
fn struct_with_no_table() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    struct MyStructNamedSoYouCantInferIt {
        #[diesel(sql_type = Integer)]
        foo: i32,
        #[diesel(sql_type = Integer)]
        bar: i32,
    }

    let conn = &mut connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(conn);
    assert_eq!(Ok(MyStructNamedSoYouCantInferIt { foo: 1, bar: 2 }), data);
}

// #[test]
// fn struct_with_non_ident_column_name() {
//     #[derive(Debug, Clone, PartialEq, Eq, QueryableByName)]
//     struct QueryPlan {
//         #[diesel(sql_type = diesel::sql_types::Text)]
//         #[diesel(column_name = "QUERY PLAN")]
//         qp: String,
//     }

//     let conn = &mut connection();
//     let data = sql_query("SELECT 'some plan' AS \"QUERY PLAN\"").get_result(conn);
//     assert_eq!(
//         Ok(QueryPlan {
//             qp: "some plan".to_string()
//         }),
//         data
//     );
// }

#[test]
fn embedded_struct() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = my_structs)]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: B,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = my_structs)]
    struct B {
        bar: i32,
    }

    let conn = &mut connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(conn);
    assert_eq!(
        Ok(A {
            foo: 1,
            b: B { bar: 2 },
        }),
        data
    );
}

#[test]
fn embedded_option() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = my_structs)]
    struct A {
        foo: i32,
        #[diesel(embed)]
        b: Option<B>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, QueryableByName)]
    #[diesel(table_name = my_structs)]
    struct B {
        bar: i32,
    }

    let conn = &mut connection();
    let data = sql_query("SELECT 1 AS foo, 2 AS bar").get_result(conn);
    assert_eq!(
        Ok(A {
            foo: 1,
            b: Some(B { bar: 2 }),
        }),
        data
    );
    let data = sql_query("SELECT 1 AS foo, NULL AS bar").get_result(conn);
    assert_eq!(Ok(A { foo: 1, b: None }), data);
}
