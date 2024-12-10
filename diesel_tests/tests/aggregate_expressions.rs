#![cfg(feature = "postgres")] // todo
use crate::schema::connection_with_sean_and_tess_in_users_table;
use crate::schema::users;
use diesel::dsl::{
    self, frame, AggregateExpressionMethods, FrameBoundDsl, FrameClauseDsl, WindowExpressionMethods,
};
use diesel::prelude::*;

#[test]
fn test1() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let query = users::table.select(dsl::count(users::id).filter_aggregate(users::name.eq("Sean")));
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query));

    let res = query.get_result::<i64>(&mut conn).unwrap();
    assert_eq!(res, 1);

    let query2 = users::table.select(
        dsl::count(users::id)
            .distinct()
            .filter_aggregate(users::name.eq("Sean")),
    );
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query2));
    let res = query2.get_result::<i64>(&mut conn).unwrap();
    dbg!(res);

    let query3 = users::table.select(
        dsl::count(users::id)
            .distinct()
            .filter_aggregate(users::name.eq("Sean"))
            .order_aggregate(users::id),
    );
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query3));
    let res = query3.get_result::<i64>(&mut conn).unwrap();
    dbg!(res);
    todo!()
}

#[test]
fn test2() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let query = users::table.select(dsl::count(users::id).over());
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query));

    let res = query.get_result::<i64>(&mut conn).unwrap();
    assert_eq!(res, 2);

    let query = users::table.select(dsl::count(users::id).over().partition_by(users::name));
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query));

    let res = query.get_result::<i64>(&mut conn).unwrap();
    assert_eq!(res, 1);

    let query = users::table.select(dsl::count(users::id).over().window_order(users::name));
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query));

    let res = query.get_result::<i64>(&mut conn).unwrap();
    assert_eq!(res, 1);

    let query = users::table.select(
        dsl::count(users::id)
            .over()
            .window_order(users::name)
            .partition_by(users::name)
            .frame_by(frame::Rows.start_with(2.preceding())),
    );
    dbg!(diesel::debug_query::<diesel::pg::Pg, _>(&query));

    let res = query.get_result::<i64>(&mut conn).unwrap();
    assert_eq!(res, 1);
    todo!()
}
