use schema::*;
use diesel::*;

#[test]
// This test is a shim for a feature which is not sufficiently implemented. It
// has been added as we have a user who needs a reasonable workaround, but this
// functionality will change and this test is allowed to change post-1.0
fn group_by_generates_group_by_sql() {
    let source = users::table.group_by(users::name).select(users::id).filter(users::hair_color.is_null());
    let expected_sql = "SELECT `users`.`id` FROM `users` \
                        WHERE `users`.`hair_color` IS NULL \
                        GROUP BY `users`.`name`";

    assert_eq!(expected_sql, &debug_sql!(source));
}
