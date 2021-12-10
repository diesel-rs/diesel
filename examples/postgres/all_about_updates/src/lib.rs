use std::time::SystemTime;

#[cfg(test)]
use diesel::debug_query;
#[cfg(test)]
use diesel::pg::Pg;
use diesel::prelude::*;

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

pub fn publish_all_posts(conn: &mut PgConnection) -> QueryResult<usize> {
    use crate::posts::dsl::*;

    diesel::update(posts).set(draft.eq(false)).execute(conn)
}

#[test]
fn examine_sql_from_publish_all_posts() {
    use crate::posts::dsl::*;

    assert_eq!(
        "UPDATE \"posts\" SET \"draft\" = $1 -- binds: [false]",
        debug_query(&diesel::update(posts).set(draft.eq(false))).to_string()
    );
}

pub fn publish_pending_posts(conn: &mut PgConnection) -> QueryResult<usize> {
    use crate::posts::dsl::*;
    use diesel::dsl::now;

    diesel::update(posts)
        .filter(publish_at.lt(now))
        .set(draft.eq(false))
        .execute(conn)
}

#[test]
fn examine_sql_from_publish_pending_posts() {
    use crate::posts::dsl::*;
    use diesel::dsl::now;

    let query = diesel::update(posts)
        .filter(publish_at.lt(now))
        .set(draft.eq(false));
    assert_eq!(
        "UPDATE \"posts\" SET \"draft\" = $1 \
         WHERE (\"posts\".\"publish_at\" < CURRENT_TIMESTAMP) \
         -- binds: [false]",
        debug_query(&query).to_string()
    );
}

pub fn publish_post(post: &Post, conn: &mut PgConnection) -> QueryResult<usize> {
    diesel::update(post)
        .set(posts::draft.eq(false))
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
        "UPDATE \"posts\" SET \"draft\" = $1 WHERE (\"posts\".\"id\" = $2) \
         -- binds: [false, 1]",
        debug_query(&diesel::update(&post).set(posts::draft.eq(false))).to_string()
    );
}

pub fn increment_visit_counts(conn: &mut PgConnection) -> QueryResult<usize> {
    use crate::posts::dsl::*;

    diesel::update(posts)
        .set(visit_count.eq(visit_count + 1))
        .execute(conn)
}

#[test]
fn examine_sql_from_increment_visit_counts() {
    use crate::posts::dsl::*;

    assert_eq!(
        "UPDATE \"posts\" SET \"visit_count\" = (\"posts\".\"visit_count\" + $1) \
         -- binds: [1]",
        debug_query::<Pg, _>(&diesel::update(posts).set(visit_count.eq(visit_count + 1)))
            .to_string()
    );
}

pub fn hide_everything(conn: &mut PgConnection) -> QueryResult<usize> {
    use crate::posts::dsl::*;

    diesel::update(posts)
        .set((
            title.eq("[REDACTED]"),
            body.eq("This post has been classified"),
        ))
        .execute(conn)
}

#[test]
fn examine_sql_from_hide_everything() {
    use crate::posts::dsl::*;

    let query = diesel::update(posts).set((
        title.eq("[REDACTED]"),
        body.eq("This post has been classified"),
    ));
    assert_eq!(
        "UPDATE \"posts\" SET \"title\" = $1, \"body\" = $2 \
         -- binds: [\"[REDACTED]\", \"This post has been classified\"]",
        debug_query::<Pg, _>(&query).to_string()
    );
}

pub fn update_from_post_fields(post: &Post, conn: &mut PgConnection) -> QueryResult<usize> {
    diesel::update(posts::table).set(post).execute(conn)
}

#[test]
fn examine_sql_from_update_post_fields() {
    let now = SystemTime::now();
    let post = Post {
        id: 1,
        title: "".into(),
        body: "".into(),
        draft: false,
        publish_at: now,
        visit_count: 0,
    };
    let sql = format!(
        "UPDATE \"posts\" SET \
         \"title\" = $1, \
         \"body\" = $2, \
         \"draft\" = $3, \
         \"publish_at\" = $4, \
         \"visit_count\" = $5 \
         -- binds: [\
         \"\", \
         \"\", \
         false, \
         {:?}, \
         0\
         ]",
        now
    );
    assert_eq!(
        sql,
        debug_query(&diesel::update(posts::table).set(&post)).to_string()
    );
}

pub fn update_with_option(conn: &mut PgConnection) -> QueryResult<usize> {
    #[derive(AsChangeset)]
    #[diesel(table_name = posts)]
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
    #[diesel(table_name = posts)]
    struct PostForm<'a> {
        title: Option<&'a str>,
        body: Option<&'a str>,
    }

    let post_form = PostForm {
        title: None,
        body: Some("My new post"),
    };
    let query = diesel::update(posts::table).set(&post_form);
    assert_eq!(
        "UPDATE \"posts\" SET \"body\" = $1 \
         -- binds: [\"My new post\"]",
        debug_query::<Pg, _>(&query).to_string()
    );
}
