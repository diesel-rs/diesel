use crate::schema::TestConnection;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;

diesel::view! {
    view {
        not_id -> Integer,
        name -> Text,
    }
}

diesel::view! {
    view2 {
        not_view_id -> Integer,
        title -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(view, view2);

fn view_test_setup() -> TestConnection {
    let mut conn = crate::schema::connection();
    #[cfg(not(feature = "mysql"))]
    conn.batch_execute(
        "CREATE TEMPORARY VIEW view AS SELECT 42 AS not_id, 'John' AS name; \
          CREATE TEMPORARY VIEW view2 AS SELECT 42 AS not_view_id, 'views!!' AS title;",
    )
    .unwrap();

    #[cfg(feature = "mysql")]
    conn.batch_execute(
        "CREATE OR REPLACE VIEW view AS SELECT 42 AS not_id, 'John' AS name; \
          CREATE OR REPLACE VIEW view2 AS SELECT 42 AS not_view_id, 'views!!' AS title;",
    )
    .unwrap();

    conn
}

// todo: Write compile fail tests for cases that should not compile as for example:
//     - `view.find(42)` (No primary key)
//     - Insert
//     - Update
//     - Delete
//     - ? (Test what other methods to disallow)
#[test]
fn basic_query() {
    let conn = &mut view_test_setup();

    let res = view::view.load::<(i32, String)>(conn).unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn filter() {
    let conn = &mut view_test_setup();
    let res = view::view
        .filter(view::not_id.eq(42))
        .load::<(i32, String)>(conn)
        .unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn limit() {
    let conn = &mut view_test_setup();
    let res = view::view.limit(1).load::<(i32, String)>(conn).unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn limit_offset() {
    let conn = &mut view_test_setup();
    let res = view::view
        .limit(1)
        .offset(0)
        .load::<(i32, String)>(conn)
        .unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn order() {
    let conn = &mut view_test_setup();
    let res = view::view
        .order(view::name)
        .load::<(i32, String)>(conn)
        .unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn group_by() {
    let conn = &mut view_test_setup();
    let res = view::view
        .group_by((view::not_id, view::name))
        .load::<(i32, String)>(conn)
        .unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn inner_join() {
    let conn = &mut view_test_setup();
    let res = view::view
        .inner_join(view2::view.on(view::not_id.eq(view2::not_view_id)))
        .load::<((i32, String), (i32, String))>(conn)
        .unwrap();

    assert_eq!(res, [((42, "John".to_string()), (42, "views!!".into()))]);
}

#[test]
fn left_join() {
    let conn = &mut view_test_setup();
    let res = view::view
        .left_join(view2::view.on(view::not_id.eq(view2::not_view_id)))
        .load::<((i32, String), Option<(i32, String)>)>(conn)
        .unwrap();

    assert_eq!(
        res,
        [((42, "John".to_string()), Some((42, "views!!".into())))]
    );
}

#[test]
fn distinct() {
    let conn = &mut view_test_setup();

    let res = view::view.distinct().load::<(i32, String)>(conn).unwrap();

    assert_eq!(res, [(42, "John".into())]);
}

#[test]
fn count() {
    let conn = &mut view_test_setup();

    let res = view::view.count().get_result::<i64>(conn).unwrap();

    assert_eq!(res, 1);
}

#[test]
fn first() {
    let conn = &mut view_test_setup();

    let res = view::view.first::<(i32, String)>(conn).unwrap();

    assert_eq!(res, (42, "John".into()));
}

#[test]
fn alias() {
    let conn = &mut view_test_setup();
    let view_alias = diesel::alias!(view as view1);

    let name = view_alias
        .select(view_alias.field(view::name))
        .first::<String>(conn)
        .unwrap();
    assert_eq!(name, "John");
}

// todo: test larger parts of `QueryDsl`
// todo: lock dsl
// todo: having
// todo: double check what else is missing
