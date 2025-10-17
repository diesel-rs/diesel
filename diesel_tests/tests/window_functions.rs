use crate::schema::connection_with_sean_and_tess_in_users_table;
use crate::schema::{posts, users};
use diesel::dsl;
use diesel::prelude::*;

#[diesel_test_helper::test]
fn simple_window() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(dsl::count(users::id).over())
        .load::<i64>(&mut conn)
        .unwrap();

    assert_eq!(res, vec![2, 2]);
}

#[diesel_test_helper::test]
fn window_count_star() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(dsl::count_star().over())
        .load::<i64>(&mut conn)
        .unwrap();

    assert_eq!(res, vec![2, 2]);
}

#[diesel_test_helper::test]
fn partition_by() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    diesel::insert_into(posts::table)
        .values([
            (posts::title.eq("Post 1"), posts::user_id.eq(1)),
            (posts::title.eq("Post 2"), posts::user_id.eq(1)),
            (posts::title.eq("Post 3"), posts::user_id.eq(2)),
        ])
        .execute(&mut conn)
        .unwrap();

    let res = users::table
        .inner_join(posts::table)
        .select((users::name, dsl::count(posts::id).partition_by(users::id)))
        .order_by(users::id)
        .load::<(String, i64)>(&mut conn)
        .unwrap();

    assert_eq!(
        res,
        vec![
            (String::from("Sean"), 2),
            (String::from("Sean"), 2),
            (String::from("Tess"), 1)
        ]
    );
}

// that's not meaningful to test with
// only `count` as test function for now
// TODO: change the used function here
// as soon as we have `lead`/`lag`/`first`
#[diesel_test_helper::test]
fn order_smoke_test() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(dsl::count(users::id).window_order(users::name))
        .order_by(users::id)
        .load::<i64>(&mut conn)
        .unwrap();

    assert_eq!(res, vec![1, 2]);
}

#[diesel_test_helper::test]
fn frame_no_preceding() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(dsl::count(users::id).frame_by(dsl::frame::Rows.frame_start_with(0.preceding())))
        .load::<i64>(&mut conn)
        .unwrap();

    assert_eq!(res, vec![1, 1]);
}

#[diesel_test_helper::test]
fn frame_no_following() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(dsl::count(users::id).frame_by(
            dsl::frame::Rows.frame_between(dsl::frame::UnboundedPreceding, 0.following()),
        ))
        .load::<i64>(&mut conn)
        .unwrap();

    assert_eq!(res, vec![1, 2]);
}

#[cfg(not(feature = "mysql"))] // mysql doesn't support exclusion
#[diesel_test_helper::test]
fn frame_skip_current_row() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(
            dsl::count(users::id).frame_by(dsl::frame::Rows.frame_between_with_exclusion(
                dsl::frame::UnboundedPreceding,
                dsl::frame::UnboundedFollowing,
                dsl::frame::ExcludeCurrentRow,
            )),
        )
        .load::<i64>(&mut conn)
        .unwrap();

    assert_eq!(res, vec![1, 1]);
}

#[diesel_test_helper::test]
fn frame_current_row_only() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(
            dsl::count(users::id)
                .frame_by(dsl::frame::Rows.frame_start_with(dsl::frame::CurrentRow)),
        )
        .load::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![1, 1]);
}

#[cfg(not(feature = "mysql"))] // mysql doesn't support exclusion
#[diesel_test_helper::test]
fn frame_current_row_exclude_current_row() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res =
        users::table
            .select(dsl::count(users::id).frame_by(
                dsl::frame::Rows.frame_start_with_exclusion(
                    dsl::frame::CurrentRow,
                    dsl::frame::ExcludeCurrentRow,
                ),
            ))
            .load::<i64>(&mut conn)
            .unwrap();
    assert_eq!(res, vec![0, 0]);
}

#[cfg(not(feature = "mysql"))] // mysql doesn't support exclusion
#[diesel_test_helper::test]
fn frame_current_row_exclude_group() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(
            dsl::count(users::id).frame_by(
                dsl::frame::Rows
                    .frame_start_with_exclusion(dsl::frame::CurrentRow, dsl::frame::ExcludeGroup),
            ),
        )
        .load::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![0, 0]);
}

#[cfg(not(feature = "mysql"))] // mysql doesn't support exclusion
#[diesel_test_helper::test]
fn frame_current_row_exclude_ties() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(
            dsl::count(users::id).frame_by(
                dsl::frame::Rows
                    .frame_start_with_exclusion(dsl::frame::CurrentRow, dsl::frame::ExcludeTies),
            ),
        )
        .load::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![1, 1]);
}

#[cfg(not(feature = "mysql"))] // mysql doesn't support exclusion
#[diesel_test_helper::test]
fn frame_current_row_exclude_no_others() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res =
        users::table
            .select(dsl::count(users::id).frame_by(
                dsl::frame::Rows.frame_start_with_exclusion(
                    dsl::frame::CurrentRow,
                    dsl::frame::ExcludeNoOthers,
                ),
            ))
            .load::<i64>(&mut conn)
            .unwrap();
    assert_eq!(res, vec![1, 1]);
}

#[diesel_test_helper::test]
fn frame_range() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(
            dsl::count(users::id)
                .frame_by(dsl::frame::Range.frame_start_with(dsl::frame::UnboundedPreceding)),
        )
        .load::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![2, 2]);
}

#[cfg(not(feature = "mysql"))] // mysql doesn't support group frame clauses
#[diesel_test_helper::test]
fn frame_groups() {
    let mut conn = connection_with_sean_and_tess_in_users_table();

    let res = users::table
        .select(
            dsl::count(users::id)
                .window_order(users::name)
                .frame_by(dsl::frame::Groups.frame_start_with(dsl::frame::UnboundedPreceding)),
        )
        .load::<i64>(&mut conn)
        .unwrap();
    assert_eq!(res, vec![1, 2]);
}
