use crate::consts::build_insert_users_params;
use crate::consts::mysql::{
    build_insert_users_query, CLEANUP_QUERIES, MEDIUM_COMPLEX_QUERY_BY_ID, TRIVIAL_QUERY,
};
use crate::Bencher;
use std::collections::HashMap;
use std::fmt::Write;
use tokio::runtime::Runtime;
use zero_mysql::r#macro::FromRawRow;
use zero_mysql::tokio::Conn;
use zero_mysql::Opts;

#[derive(FromRawRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

#[derive(FromRawRow)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

#[derive(FromRawRow)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub text: String,
}

async fn connection() -> Conn {
    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("MYSQL_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut opts = Opts::try_from(connection_url.as_str()).unwrap();
    opts.upgrade_to_unix_socket = false;
    let mut conn: Conn = Conn::new(opts).await.unwrap();

    conn.query_drop(&CLEANUP_QUERIES.join("; ")).await.unwrap();

    conn
}

async fn insert_users_for_setup(
    conn: &mut Conn,
    size: usize,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let query = build_insert_users_query(size);
    let params: Vec<zero_mysql::Value> = build_insert_users_params(size, hair_color_init)
        .into_iter()
        .flat_map(|(name, hair_color)| [name.into(), hair_color.into()])
        .collect();
    let mut stmt = conn.prepare(&query).await.unwrap();
    conn.exec_drop(&mut stmt, params).await.unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().unwrap();
    let (mut conn, mut stmt) = runtime.block_on(async {
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
            conn.exec_foreach(&mut stmt, (), |row: (i32, String, Option<String>)| {
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
    let (mut conn, mut stmt) = runtime.block_on(async {
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
            conn.exec_foreach(&mut stmt, (), |user: User| {
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
    let (mut conn, mut stmt) = runtime.block_on(async {
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
                &mut stmt,
                ("black",),
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
    let (mut conn, mut stmt) = runtime.block_on(async {
        let mut conn = connection().await;
        let query = build_insert_users_query(size);
        let stmt = conn.prepare(&query).await.unwrap();
        (conn, stmt)
    });

    b.iter(|| {
        runtime.block_on(async {
            let params: Vec<zero_mysql::Value> =
                build_insert_users_params(size, |_| Some("hair_color"))
                    .into_iter()
                    .flat_map(|(name, hair_color)| [name.into(), hair_color.into()])
                    .collect();
            conn.exec_drop(&mut stmt, params).await.unwrap();
        })
    })
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    let runtime = Runtime::new().unwrap();
    let (mut conn, mut user_stmt) = runtime.block_on(async {
        let mut conn = connection().await;

        insert_users_for_setup(&mut conn, 100, |i| {
            Some(if i % 2 == 0 { "black" } else { "brown" })
        })
        .await;

        let mut user_ids = Vec::new();
        let mut stmt = conn.prepare("SELECT id FROM users").await.unwrap();
        conn.exec_foreach(&mut stmt, (), |row: (i32,)| {
            user_ids.push(row.0);
            Ok(())
        })
        .await
        .unwrap();

        let mut insert_query = String::from("INSERT INTO posts(title, user_id, body) VALUES ");
        for (idx, user_id) in user_ids.iter().enumerate() {
            for i in 0..10 {
                if idx > 0 || i > 0 {
                    insert_query.push(',');
                }
                write!(
                    insert_query,
                    "('Post {} by user {}', {}, NULL)",
                    i, user_id, user_id
                )
                .unwrap();
            }
        }
        conn.query_drop(&insert_query).await.unwrap();

        let mut all_posts = Vec::new();
        let mut stmt = conn.prepare("SELECT id FROM posts").await.unwrap();
        conn.exec_foreach(&mut stmt, (), |row: (i32,)| {
            all_posts.push(row.0);
            Ok(())
        })
        .await
        .unwrap();

        let mut insert_query = String::from("INSERT INTO comments(text, post_id) VALUES ");
        for (idx, post_id) in all_posts.iter().enumerate() {
            for i in 0..10 {
                if idx > 0 || i > 0 {
                    insert_query.push(',');
                }
                write!(
                    insert_query,
                    "('Comment {} on post {}', {})",
                    i, post_id, post_id
                )
                .unwrap();
            }
        }
        conn.query_drop(&insert_query).await.unwrap();

        let user_stmt = conn
            .prepare(TRIVIAL_QUERY)
            .await
            .unwrap();

        (conn, user_stmt)
    });

    b.iter(|| {
        runtime.block_on(async {
            let mut users = Vec::new();
            conn.exec_foreach(&mut user_stmt, (), |user: User| {
                users.push(user);
                Ok(())
            })
            .await
            .unwrap();

            let user_ids: Vec<i32> = users.iter().map(|u| u.id).collect();
            let user_ids_str = user_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let posts_query = format!(
                "SELECT id, user_id, title, body FROM posts WHERE user_id IN({})",
                user_ids_str
            );

            let mut posts = Vec::new();
            let mut stmt = conn.prepare(&posts_query).await.unwrap();
            conn.exec_foreach(&mut stmt, (), |post: Post| {
                posts.push(post);
                Ok(())
            })
            .await
            .unwrap();

            let post_ids: Vec<i32> = posts.iter().map(|p| p.id).collect();
            let post_ids_str = post_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let comments_query = format!(
                "SELECT id, post_id, text FROM comments WHERE post_id IN({})",
                post_ids_str
            );

            let mut comments = Vec::new();
            let mut stmt = conn.prepare(&comments_query).await.unwrap();
            conn.exec_foreach(&mut stmt, (), |comment: Comment| {
                comments.push(comment);
                Ok(())
            })
            .await
            .unwrap();

            let mut posts = posts
                .into_iter()
                .map(|p| (p.id, (p, Vec::new())))
                .collect::<HashMap<_, _>>();

            let mut users = users
                .into_iter()
                .map(|u| (u.id, (u, Vec::new())))
                .collect::<HashMap<_, _>>();

            for comment in comments {
                posts.get_mut(&comment.post_id).unwrap().1.push(comment);
            }

            for (_, post_with_comments) in posts {
                users
                    .get_mut(&post_with_comments.0.user_id)
                    .unwrap()
                    .1
                    .push(post_with_comments);
            }

            users
                .into_iter()
                .map(|(_, users_with_post_and_comment)| users_with_post_and_comment)
                .collect::<Vec<(User, Vec<(Post, Vec<Comment>)>)>>()
        })
    })
}
