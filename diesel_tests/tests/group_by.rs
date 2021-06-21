use crate::schema::*;
use diesel::*;

#[test]
// This test is a shim for a feature which is not sufficiently implemented. It
// has been added as we have a user who needs a reasonable workaround, but this
// functionality will change and this test is allowed to change post-1.0
fn group_by_generates_group_by_sql() {
    let source = users::table
        .group_by(users::name)
        .select(users::name)
        .filter(users::hair_color.is_null());
    let mut expected_sql = "SELECT `users`.`name` FROM `users` \
                            WHERE (`users`.`hair_color` IS NULL) \
                            GROUP BY `users`.`name` \
                            -- binds: []"
        .to_string();
    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(
        expected_sql,
        debug_query::<TestBackend, _>(&source).to_string()
    );
    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn group_by_mixed_aggregate_column_and_aggregate_function() {
    use diesel::dsl::max;
    let source = users::table
        .group_by(users::name)
        .select((max(users::id), users::name))
        .filter(users::hair_color.is_null());
    let mut expected_sql = "SELECT max(`users`.`id`), `users`.`name` FROM `users` \
                            WHERE (`users`.`hair_color` IS NULL) \
                            GROUP BY `users`.`name` \
                            -- binds: []"
        .to_string();
    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(
        expected_sql,
        debug_query::<TestBackend, _>(&source).to_string()
    );

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn boxed_queries_have_group_by_method() {
    let source = users::table
        .group_by(users::name)
        .select(users::name)
        .into_boxed::<TestBackend>()
        .filter(users::hair_color.is_null());
    let mut expected_sql = "SELECT `users`.`name` FROM `users` \
                            WHERE (`users`.`hair_color` IS NULL) \
                            GROUP BY `users`.`name` \
                            -- binds: []"
        .to_string();
    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn check_group_by_primary_key_allows_other_columns_in_select_clause() {
    let source = users::table
        .group_by(users::id)
        .select(users::name)
        .filter(users::hair_color.is_null());

    let mut expected_sql = "SELECT `users`.`name` FROM `users` \
                            WHERE (`users`.`hair_color` IS NULL) \
                            GROUP BY `users`.`id` \
                            -- binds: []"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn check_group_by_multiple_columns_in_group_by_clause_single_select() {
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select(users::name)
        .filter(users::id.nullable().is_null());

    let mut expected_sql = "SELECT `users`.`name` \
                            FROM `users` WHERE (`users`.`id` IS NULL) \
                            GROUP BY `users`.`name`, `users`.`hair_color` \
                            -- binds: []"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn check_group_by_multiple_columns_in_group_by_clause_complex_select() {
    let source = users::table
        .group_by((users::name, users::hair_color))
        .select((users::name, users::hair_color, diesel::dsl::max(users::id)))
        .filter(users::id.nullable().is_null());

    let mut expected_sql = "SELECT `users`.`name`, `users`.`hair_color`, max(`users`.`id`) \
                            FROM `users` WHERE (`users`.`id` IS NULL) \
                            GROUP BY `users`.`name`, `users`.`hair_color` \
                            -- binds: []"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

diesel::allow_columns_to_appear_in_same_group_by_clause!(
    posts::id,
    users::id,
    posts::title,
    users::name
);

#[test]
fn check_group_by_multiple_tables() {
    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .select((users::name, posts::title, diesel::dsl::count(comments::id)))
        .filter(comments::text.nullable().is_null());

    let mut expected_sql = "SELECT `users`.`name`, `posts`.`title`, count(`comments`.`id`) \
                            FROM (`users` \
                            INNER JOIN (`posts` \
                            INNER JOIN `comments` ON (`comments`.`post_id` = `posts`.`id`)) \
                            ON (`posts`.`user_id` = `users`.`id`)) \
                            WHERE (`comments`.`text` IS NULL) \
                            GROUP BY `users`.`id`, `posts`.`id` \
                            -- binds: []"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn check_filter_with_group_by_subselect() {
    let subselect = posts::table
        .group_by(posts::user_id)
        .select(diesel::dsl::min(posts::id));

    // get one example post for each user (for users with a post)
    let source = posts::table
        .filter(posts::id.nullable().eq_any(subselect))
        .select((posts::user_id, posts::title));

    let mut expected_sql = "SELECT `posts`.`user_id`, `posts`.`title` \
                            FROM `posts` \
                            WHERE (\
                              `posts`.`id` IN (\
                              SELECT min(`posts`.`id`) \
                              FROM `posts` \
                              GROUP BY `posts`.`user_id`)) \
                            -- binds: []"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn check_filter_with_boxed_group_by_subselect() {
    // this query is identical to `check_filter_with_group_by_subselect`, but the subselect also calls `into_boxed`
    let subselect = posts::table
        .group_by(posts::user_id)
        .select(diesel::dsl::min(posts::id))
        .into_boxed();

    // get one example post for each user (for users with a post)
    let source = posts::table
        .filter(posts::id.nullable().eq_any(subselect))
        .select((posts::user_id, posts::title));

    let mut expected_sql = "SELECT `posts`.`user_id`, `posts`.`title` \
                            FROM `posts` \
                            WHERE (\
                              `posts`.`id` IN (\
                              SELECT min(`posts`.`id`) \
                              FROM `posts` \
                              GROUP BY `posts`.`user_id`)) \
                            -- binds: []"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
    }

    assert_eq!(expected_sql, debug_query(&source).to_string());

    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}
