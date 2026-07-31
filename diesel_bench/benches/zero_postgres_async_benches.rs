use crate::consts::build_insert_users_params;
use crate::consts::postgres::{
    build_insert_users_query, CLEANUP_QUERIES, MEDIUM_COMPLEX_QUERY_BY_ID, TRIVIAL_QUERY,
};
use crate::Bencher;
use std::collections::HashMap;
use std::fmt::Write;
use tokio::runtime::Runtime;
use zero_postgres::r#macro::FromRow;
use zero_postgres::tokio::Conn;
use zero_postgres::Opts;

#[derive(FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

#[derive(FromRow)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

#[derive(FromRow)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub text: String,
}

async fn connection() -> Conn {
    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("POSTGRES_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut opts = Opts::try_from(connection_url.as_str()).unwrap();
    opts.upgrade_to_unix_socket = false;
    let mut conn = Conn::new(opts).await.unwrap();

    conn.query_drop(&CLEANUP_QUERIES.join("; ")).await.unwrap();

    conn
}

async fn insert_users_for_setup(
    conn: &mut Conn,
    size: usize,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let query = build_insert_users_query(size);
    let params: Vec<Option<String>> = build_insert_users_params(size, hair_color_init)
        .into_iter()
        .flat_map(|(name, hair_color)| [Some(name), hair_color.map(String::from)])
        .collect();
    let stmt = conn.prepare(&query).await.unwrap();
    conn.exec_drop(&stmt, params).await.unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().unwrap();
    let (mut conn, stmt) = runtime.block_on(async {
        let mut conn = connection().await;
        insert_users_for_setup(&mut conn, size, |_| None).await;
        let stmt = conn
            .prepare(TRIVIAL_QUERY)
            .await
            .unwrap();
        (conn, stmt)
    });

    b.iter(|| {
        runtime.block_on(async {
            let mut users = Vec::new();
            conn.exec_foreach(&stmt, (), |row: (i32, String, Option<String>)| {
                users.push(User {
                    id: row.0,
                    name: row.1,
                    hair_color: row.2,
                });
                Ok(())
            })
            .await
            .unwrap();
            users
        })
    })
}

pub fn bench_trivial_query_by_name(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().unwrap();
    let (mut conn, stmt) = runtime.block_on(async {
        let mut conn = connection().await;
        insert_users_for_setup(&mut conn, size, |_| None).await;
        let stmt = conn
            .prepare(TRIVIAL_QUERY)
            .await
            .unwrap();
        (conn, stmt)
    });

    b.iter(|| {
        runtime.block_on(async {
            let mut users = Vec::new();
            conn.exec_foreach(&stmt, (), |user: User| {
                users.push(user);
                Ok(())
            })
            .await
            .unwrap();
            users
        })
    })
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().unwrap();
    let (mut conn, stmt) = runtime.block_on(async {
        let mut conn = connection().await;
        insert_users_for_setup(&mut conn, size, |i| {
            Some(if i % 2 == 0 { "black" } else { "brown" })
        })
        .await;
        let stmt = conn.prepare(MEDIUM_COMPLEX_QUERY_BY_ID).await.unwrap();
        (conn, stmt)
    });

    b.iter(|| {
        runtime.block_on(async {
            let mut results = Vec::new();
            conn.exec_foreach(
                &stmt,
                (&"black",),
                |row: (
                    i32,
                    String,
                    Option<String>,
                    Option<i32>,
                    Option<i32>,
                    Option<String>,
                    Option<String>,
                )| {
                    let user = User {
                        id: row.0,
                        name: row.1,
                        hair_color: row.2,
                    };
                    let post = row.3.map(|id| Post {
                        id,
                        user_id: row.4.unwrap(),
                        title: row.5.unwrap(),
                        body: row.6,
                    });
                    results.push((user, post));
                    Ok(())
                },
            )
            .await
            .unwrap();
            results
        })
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().unwrap();
    let (mut conn, stmt) = runtime.block_on(async {
        let mut conn = connection().await;
        let query = build_insert_users_query(size);
        let stmt = conn.prepare(&query).await.unwrap();
        (conn, stmt)
    });

    b.iter(|| {
        runtime.block_on(async {
            let params: Vec<Option<String>> =
                build_insert_users_params(size, |_| Some("hair_color"))
                    .into_iter()
                    .flat_map(|(name, hair_color)| [Some(name), hair_color.map(String::from)])
                    .collect();
            conn.exec_drop(&stmt, params).await.unwrap();
        })
    })
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    let runtime = Runtime::new().unwrap();
    let mut conn = runtime.block_on(async {
        let mut conn = connection().await;

        insert_users_for_setup(&mut conn, 100, |i| {
            Some(if i % 2 == 0 { "black" } else { "brown" })
        })
        .await;

        // Get user IDs
        let user_ids: Vec<(i32,)> = conn.query_collect("SELECT id FROM users").await.unwrap();

        // Insert posts
        let mut insert_query = String::from("INSERT INTO posts(title, user_id, body) VALUES ");
        let mut first = true;
        for (user_id,) in &user_ids {
            for i in 0..10 {
                if !first {
                    insert_query.push(',');
                }
                first = false;
                write!(
                    insert_query,
                    "('Post {} by user {}', {}, NULL)",
                    i, user_id, user_id
                )
                .unwrap();
            }
        }
        conn.query_drop(&insert_query).await.unwrap();

        // Get post IDs
        let all_posts: Vec<(i32,)> = conn.query_collect("SELECT id FROM posts").await.unwrap();

        // Insert comments
        let mut insert_query = String::from("INSERT INTO comments(text, post_id) VALUES ");
        let mut first = true;
        for (post_id,) in &all_posts {
            for i in 0..10 {
                if !first {
                    insert_query.push(',');
                }
                first = false;
                write!(
                    insert_query,
                    "('Comment {} on post {}', {})",
                    i, post_id, post_id
                )
                .unwrap();
            }
        }
        conn.query_drop(&insert_query).await.unwrap();

        conn
    });

    b.iter(|| {
        runtime.block_on(async {
            // Load users
            let users: Vec<User> = conn
                .query_collect(TRIVIAL_QUERY)
                .await
                .unwrap();

            // Build IN clause with actual values
            let user_ids_str = users
                .iter()
                .map(|u| u.id.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let posts_query = format!(
                "SELECT id, user_id, title, body FROM posts WHERE user_id IN({})",
                user_ids_str
            );
            let posts: Vec<Post> = conn.query_collect(&posts_query).await.unwrap();

            let post_ids_str = posts
                .iter()
                .map(|p| p.id.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let comments_query = format!(
                "SELECT id, post_id, text FROM comments WHERE post_id IN({})",
                post_ids_str
            );
            let comments: Vec<Comment> = conn.query_collect(&comments_query).await.unwrap();

            // Build the nested structure
            let mut posts_map = posts
                .into_iter()
                .map(|p| (p.id, (p, Vec::new())))
                .collect::<HashMap<_, _>>();

            let mut users_map = users
                .into_iter()
                .map(|u| (u.id, (u, Vec::new())))
                .collect::<HashMap<_, _>>();

            for comment in comments {
                posts_map.get_mut(&comment.post_id).unwrap().1.push(comment);
            }

            for (_, post_with_comments) in posts_map {
                users_map
                    .get_mut(&post_with_comments.0.user_id)
                    .unwrap()
                    .1
                    .push(post_with_comments);
            }

            users_map
                .into_iter()
                .map(|(_, users_with_post_and_comment)| users_with_post_and_comment)
                .collect::<Vec<(User, Vec<(Post, Vec<Comment>)>)>>()
        })
    })
}
