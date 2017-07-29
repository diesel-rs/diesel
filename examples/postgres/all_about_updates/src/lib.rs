#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

use std::time::SystemTime;

use diesel::prelude::*;
use diesel::pg::PgConnection;

table! {
    posts {
        id -> BigInt,
        title -> Text,
        body -> Text,
        draft -> Bool,
        publish_at -> Timestamp,
        visit_count -> Integer,
    }
}

#[derive(Queryable, Identifiable, AsChangeset)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub draft: bool,
    pub publish_at: SystemTime,
    pub visit_count: i32,
}

pub fn publish_all_posts(conn: &PgConnection) -> QueryResult<usize> {
    use posts::dsl::*;

    diesel::update(posts).set(draft.eq(false))
        .execute(conn)
}

#[test]
fn examine_sql_from_publish_all_posts() {
    use posts::dsl::*;

    assert_eq!(
        "UPDATE `posts` SET `draft` = ?".to_string(),
        debug_sql!(diesel::update(posts).set(draft.eq(false)))
    );
}

pub fn publish_pending_posts(conn: &PgConnection) -> QueryResult<usize> {
    use posts::dsl::*;
    use diesel::expression::dsl::now;

    let target = posts.filter(publish_at.lt(now));
    diesel::update(target).set(draft.eq(false))
        .execute(conn)
}

#[test]
fn examine_sql_from_publish_pending_posts() {
    use posts::dsl::*;
    use diesel::expression::dsl::now;

    let target = posts.filter(publish_at.lt(now));
    assert_eq!(
        "UPDATE `posts` SET `draft` = ? \
        WHERE `posts`.`publish_at` < CURRENT_TIMESTAMP".to_string(),
        debug_sql!(diesel::update(target).set(draft.eq(false)))
    );
}

pub fn publish_post(post: Post, conn: &PgConnection) -> QueryResult<usize> {
    diesel::update(&post).set(posts::draft.eq(false))
        .execute(conn)
}

#[test]
fn examine_sql_from_publish_post() {
    let post = Post {
        id: 1,
        title: "".into(),
        body: "".into(),
        draft: false,
        publish_at: SystemTime::now(),
        visit_count: 0,
    };
    assert_eq!(
        "UPDATE `posts` SET `draft` = ? WHERE `posts`.`id` = ?".to_string(),
        debug_sql!(diesel::update(&post).set(posts::draft.eq(false)))
    );
}

pub fn increment_visit_counts(conn: &PgConnection) -> QueryResult<usize> {
    use posts::dsl::*;

    diesel::update(posts).set(visit_count.eq(visit_count + 1))
        .execute(conn)
}

#[test]
fn examine_sql_from_increment_visit_counts() {
    use posts::dsl::*;

    assert_eq!(
        "UPDATE `posts` SET `visit_count` = `posts`.`visit_count` + ?".to_string(),
        debug_sql!(diesel::update(posts).set(visit_count.eq(visit_count + 1)))
    );
}

pub fn hide_everything(conn: &PgConnection) -> QueryResult<usize> {
    use posts::dsl::*;

    diesel::update(posts)
        .set((
            title.eq("[REDACTED]"),
            body.eq("This post has been classified"),
        ))
        .execute(conn)
}

#[test]
fn examine_sql_from_hide_everything() {
    use posts::dsl::*;

    let query = diesel::update(posts).set((
        title.eq("[REDACTED]"),
        body.eq("This post has been classified"),
    ));
    assert_eq!(
        "UPDATE `posts` SET `title` = ?, `body` = ?".to_string(),
        debug_sql!(query)
    );
}

pub fn update_from_post_fields(post: Post, conn: &PgConnection) -> QueryResult<usize> {
    diesel::update(posts::table).set(&post)
        .execute(conn)
}

#[test]
fn examine_sql_from_update_post_fields() {
    let post = Post {
        id: 1,
        title: "".into(),
        body: "".into(),
        draft: false,
        publish_at: SystemTime::now(),
        visit_count: 0,
    };
    assert_eq!(
        "UPDATE `posts` SET \
            `title` = ?, \
            `body` = ?, \
            `draft` = ?, \
            `publish_at` = ?, \
            `visit_count` = ?".to_string(),
        debug_sql!(diesel::update(posts::table).set(&post))
    );
}

pub fn update_with_option(conn: &PgConnection) -> QueryResult<usize> {
    #[derive(AsChangeset)]
    #[table_name="posts"]
    struct PostForm<'a> {
        title: Option<&'a str>,
        body: Option<&'a str>,
    }

    diesel::update(posts::table)
        .set(&PostForm {
            title: None,
            body: Some("My new post"),
        })
        .execute(conn)
}

#[test]
fn examine_sql_from_update_with_option() {
    #[derive(AsChangeset)]
    #[table_name="posts"]
    struct PostForm<'a> {
        title: Option<&'a str>,
        body: Option<&'a str>,
    }

    let post_form = PostForm {
        title: None,
        body: Some("My new post"),
    };
    let query = diesel::update(posts::table)
        .set(&post_form);
    assert_eq!(
        "UPDATE `posts` SET `body` = ?".to_string(),
        debug_sql!(query)
    );
}
