use crate::schema::connection_with_sean_and_tess_in_users_table;
use crate::schema::posts;
use crate::schema::users;
use diesel::dsl;
use diesel::prelude::*;

#[diesel_test_helper::test]
fn order_by_non_aggregate() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .select(users::id)
        .order_by(users::name)
        .load::<i32>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![1, 2]);
}

#[diesel_test_helper::test]
fn order_by_non_aggregate_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .order_by(users::name)
        .select(users::name)
        .load::<String>(&mut conn)
        .unwrap();
    assert_eq!(res, vec!["Sean", "Tess"]);
}

#[diesel_test_helper::test]
fn order_by_grouped_col_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .order_by(users::name)
        .select((users::name, dsl::max(users::id)))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn order_by_aggregate_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .order_by(dsl::max(users::id))
        .select((users::name, dsl::max(users::id)))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn order_by_aggregate_before_grouped_col_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    // Ordering by aggregate, selecting only the grouped column
    let res = users::table
        .group_by(users::name)
        .order_by(dsl::max(users::id))
        .select(users::name)
        .load::<String>(&mut conn)
        .unwrap();
    assert_eq!(res, vec!["Sean", "Tess"]);
}

#[diesel_test_helper::test]
fn then_order_by_aggregate_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .order_by(users::name)
        .then_order_by(dsl::max(users::id))
        .select((users::name, dsl::max(users::id)))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn then_order_by_grouped_col_after_aggregate_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .order_by(dsl::max(users::id))
        .then_order_by(users::name)
        .select((users::name, dsl::max(users::id)))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn then_order_by_grouped_col_after_aggregate_select_first() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .select((users::name, dsl::max(users::id)))
        .order_by(dsl::max(users::id))
        .then_order_by(users::name)
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn order_by_aggregate_after_select_with_group_by() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .select((users::name, dsl::max(users::id)))
        .order_by(dsl::max(users::id))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn then_order_by_aggregate_after_select_with_group_by() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .select((users::name, dsl::max(users::id)))
        .order_by(users::name)
        .then_order_by(dsl::max(users::id))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn order_by_aggregate_multi_col_group_by_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by((users::name, users::hair_color))
        .order_by(dsl::max(users::id))
        .select((users::name, dsl::max(users::id)))
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn order_by_aggregate_with_left_join_and_group_by() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    diesel::insert_into(posts::table)
        .values([
            (posts::user_id.eq(1), posts::title.eq("Sean post 1")),
            (posts::user_id.eq(1), posts::title.eq("Sean post 2")),
            (posts::user_id.eq(2), posts::title.eq("Tess post 1")),
        ])
        .execute(&mut conn)
        .unwrap();
    let res = users::table
        .left_join(posts::table)
        .group_by(users::id)
        .order_by(dsl::count(posts::id.nullable()))
        .select((users::id, dsl::count(posts::id.nullable())))
        .load::<(i32, i64)>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![(2, 1), (1, 2)]);
}

#[diesel_test_helper::test]
fn order_by_grouped_col_with_left_join_before_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    diesel::insert_into(posts::table)
        .values([
            (posts::user_id.eq(1), posts::title.eq("Sean post 1")),
            (posts::user_id.eq(1), posts::title.eq("Sean post 2")),
            (posts::user_id.eq(2), posts::title.eq("Tess post 1")),
        ])
        .execute(&mut conn)
        .unwrap();
    let res = users::table
        .left_join(posts::table)
        .group_by(users::id)
        .order_by(users::id)
        .select((users::id, dsl::count(posts::id.nullable())))
        .load::<(i32, i64)>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![(1, 2), (2, 1)]);
}

#[diesel_test_helper::test]
fn then_order_by_aggregate_with_left_join() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    diesel::insert_into(posts::table)
        .values([
            (posts::user_id.eq(1), posts::title.eq("Sean post 1")),
            (posts::user_id.eq(1), posts::title.eq("Sean post 2")),
            (posts::user_id.eq(2), posts::title.eq("Tess post 1")),
        ])
        .execute(&mut conn)
        .unwrap();
    let res = users::table
        .left_join(posts::table)
        .group_by(users::id)
        .order_by(users::id)
        .then_order_by(dsl::count(posts::id.nullable()))
        .select((users::id, dsl::count(posts::id.nullable())))
        .load::<(i32, i64)>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![(1, 2), (2, 1)]);
}

#[diesel_test_helper::test]
fn then_order_by_grouped_col_after_aggregate_with_left_join() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    diesel::insert_into(posts::table)
        .values([
            (posts::user_id.eq(1), posts::title.eq("Sean post 1")),
            (posts::user_id.eq(1), posts::title.eq("Sean post 2")),
            (posts::user_id.eq(2), posts::title.eq("Tess post 1")),
        ])
        .execute(&mut conn)
        .unwrap();
    let res = users::table
        .left_join(posts::table)
        .group_by(users::id)
        .order_by(dsl::count(posts::id.nullable()))
        .then_order_by(users::id)
        .select((users::id, dsl::count(posts::id.nullable())))
        .load::<(i32, i64)>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![(2, 1), (1, 2)]);
}

#[diesel_test_helper::test]
fn count_distinct() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    diesel::insert_into(posts::table)
        .values([
            (posts::user_id.eq(1), posts::title.eq("Sean post 1")),
            (posts::user_id.eq(1), posts::title.eq("Sean post 2")),
            (posts::user_id.eq(2), posts::title.eq("Tess post 1")),
        ])
        .execute(&mut conn)
        .unwrap();
    let res = posts::table
        .select(dsl::count(posts::user_id).aggregate_distinct())
        .get_result::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, 2);
}

#[diesel_test_helper::test]
fn count_all() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    diesel::insert_into(posts::table)
        .values([
            (posts::user_id.eq(1), posts::title.eq("Sean post 1")),
            (posts::user_id.eq(1), posts::title.eq("Sean post 2")),
            (posts::user_id.eq(2), posts::title.eq("Tess post 1")),
        ])
        .execute(&mut conn)
        .unwrap();
    let res = posts::table
        .select(dsl::count(posts::user_id).aggregate_all())
        .get_result::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, 3);
}

#[cfg(not(feature = "mysql"))]
#[diesel_test_helper::test]
fn filter() {
    use crate::schema::users;

    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(dsl::count(users::id).aggregate_filter(users::name.eq("Sean")))
        .get_result::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, 1);
}

#[cfg(not(feature = "mysql"))]
#[diesel_test_helper::test]
fn order() {
    use crate::schema::users;

    let mut conn = connection_with_sean_and_tess_in_users_table();

    // this query is not really meaningful but should
    // at least check that the query is execute
    let q = users::table.select(dsl::count(users::id).aggregate_order(users::name));
    let sql = diesel::debug_query::<crate::schema::TestBackend, _>(&q).to_string();
    assert!(
        sql.contains("ORDER BY"),
        "Expected SQL to contain order by, but got `{sql}`"
    );
    let res = q.get_result::<i64>(&mut conn).unwrap();

    assert_eq!(res, 2);
}

#[diesel_test_helper::test]
fn order_by_aggregate_with_aggregate_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .select(dsl::max(users::id))
        .order_by(dsl::max(users::id))
        .get_result::<Option<i32>>(&mut conn)
        .unwrap();
    assert_eq!(res, Some(2));
}

#[diesel_test_helper::test]
fn order_by_group_by_column_with_aggregate_select() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .group_by(users::name)
        .select((users::name, dsl::max(users::id)))
        .order_by(users::name)
        .load::<(String, Option<i32>)>(&mut conn)
        .unwrap();
    assert_eq!(
        res,
        vec![("Sean".to_string(), Some(1)), ("Tess".to_string(), Some(2)),]
    );
}

#[diesel_test_helper::test]
fn then_order_by_aggregate() {
    let mut conn = connection_with_sean_and_tess_in_users_table();
    let res = users::table
        .select(dsl::max(users::id))
        .order_by(dsl::max(users::id))
        .then_order_by(dsl::count_star())
        .get_result::<Option<i32>>(&mut conn)
        .unwrap();
    assert_eq!(res, Some(2));
}

#[diesel_test_helper::test]
fn order_group_by_then_order_by() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .order_by(users::name)
        .group_by(users::name)
        .then_order_by(dsl::max(users::id))
        .select(dsl::max(users::id))
        .get_result::<Option<i32>>(&mut conn)
        .unwrap();
    assert_eq!(res, Some(1));
}
