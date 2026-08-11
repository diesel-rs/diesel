use chrono::NaiveDateTime;
#[cfg(test)]
use chrono::Utc;

#[cfg(test)]
use diesel::debug_query;
use diesel::prelude::*;
#[cfg(test)]
use diesel::sqlite::Sqlite;

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
    pub publish_at: NaiveDateTime,
    pub visit_count: i32,
}

pub fn publish_all_posts(conn: &mut SqliteConnection) -> QueryResult<usize> {
    use crate::posts::dsl::*;

    diesel::update(posts).set(draft.eq(false)).execute(conn)
}

#[test]
fn examine_sql_from_publish_all_posts() {
    use crate::posts::dsl::*;

    assert_eq!(
        "UPDATE `posts` SET `draft` = ? -- binds: [false]",
        debug_query(&diesel::update(posts).set(draft.eq(false))).to_string()
    );
}

pub fn publish_pending_posts(conn: &mut SqliteConnection) -> QueryResult<usize> {
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
        "UPDATE `posts` SET `draft` = ? \
		 WHERE (`posts`.`publish_at` < CURRENT_TIMESTAMP) \
         -- binds: [false]",
        debug_query(&query).to_string()
    );
}

pub fn publish_post(post: &Post, conn: &mut SqliteConnection) -> QueryResult<usize> {
    diesel::update(post)
        .set(posts::draft.eq(false))
        .execute(conn)
}

#[test]
fn examine_sql_from_publish_post() {
    let now = Utc::now().naive_utc();

    let post = Post {
        id: 1,
        title: "".into(),
        body: "".into(),
        draft: false,
        publish_at: now,
        visit_count: 0,
    };
    assert_eq!(
        "UPDATE `posts` SET `draft` = ? WHERE (`posts`.`id` = ?) \
         -- binds: [false, 1]",
        debug_query(&diesel::update(&post).set(posts::draft.eq(false))).to_string()
    );
}

pub fn increment_visit_counts(conn: &mut SqliteConnection) -> QueryResult<usize> {
    use crate::posts::dsl::*;

    diesel::update(posts)
        .set(visit_count.eq(visit_count + 1))
        .execute(conn)
}

#[test]
fn examine_sql_from_increment_visit_counts() {
    use crate::posts::dsl::*;

    assert_eq!(
        "UPDATE `posts` SET `visit_count` = (`posts`.`visit_count` + ?) \
         -- binds: [1]",
        debug_query::<Sqlite, _>(&diesel::update(posts).set(visit_count.eq(visit_count + 1)))
            .to_string()
    );
}

pub fn hide_everything(conn: &mut SqliteConnection) -> QueryResult<usize> {
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
        "UPDATE `posts` SET `title` = ?, `body` = ? \
         -- binds: [\"[REDACTED]\", \"This post has been classified\"]",
        debug_query::<Sqlite, _>(&query).to_string()
    );
}

pub fn update_from_post_fields(post: &Post, conn: &mut SqliteConnection) -> QueryResult<usize> {
    diesel::update(posts::table).set(post).execute(conn)
}

#[test]
fn examine_sql_from_update_post_fields() {
    let now = Utc::now().naive_utc();

    let post = Post {
        id: 1,
        title: "".into(),
        body: "".into(),
        draft: false,
        publish_at: now,
        visit_count: 0,
    };
    let sql = format!(
        "UPDATE `posts` SET \
         `title` = ?, \
         `body` = ?, \
         `draft` = ?, \
         `publish_at` = ?, \
         `visit_count` = ? \
         -- binds: [\
         \"\", \
         \"\", \
         false, \
         {now:?}, \
         0\
         ]"
    );
    assert_eq!(
        sql,
        debug_query(&diesel::update(posts::table).set(&post)).to_string()
    );
}

pub fn update_with_option(conn: &mut SqliteConnection) -> QueryResult<usize> {
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
        "UPDATE `posts` SET `body` = ? \
         -- binds: [\"My new post\"]",
        debug_query::<Sqlite, _>(&query).to_string()
    );
}
