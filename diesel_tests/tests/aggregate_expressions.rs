use crate::schema::connection_with_sean_and_tess_in_users_table;
use crate::schema::posts;
use diesel::dsl;
use diesel::prelude::*;

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
