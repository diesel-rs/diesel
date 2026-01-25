use crate::consts::postgres::{CLEANUP_QUERIES, MEDIUM_COMPLEX_QUERY_BY_ID};
use crate::Bencher;
use std::collections::HashMap;
use std::fmt::Write;
use zero_postgres::r#macro::FromRow;
use zero_postgres::sync::Conn;
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

fn connection() -> Conn {
    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("POSTGRES_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut opts = Opts::try_from(connection_url.as_str()).unwrap();
    opts.upgrade_to_unix_socket = false;
    let mut conn = Conn::new(opts).unwrap();

    conn.query_drop(&CLEANUP_QUERIES.join("; ")).unwrap();

    conn
}

fn insert_users(
    size: usize,
    conn: &mut Conn,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let mut query = String::from("INSERT INTO users (name, hair_color) VALUES ");
    let mut params: Vec<Option<String>> = Vec::with_capacity(size * 2);
    for x in 0..size {
        if x > 0 {
            query.push(',');
        }
        let idx = x * 2;
        write!(query, "(${}, ${})", idx + 1, idx + 2).unwrap();
        params.push(Some(format!("User {}", x)));
        params.push(hair_color_init(x).map(String::from));
    }
    let stmt = conn.prepare(&query).unwrap();
    conn.exec_drop(&stmt, params).unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let stmt = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let mut users = Vec::new();
        conn.exec_foreach(&stmt, (), |row: (i32, String, Option<String>)| {
            users.push(User {
                id: row.0,
                name: row.1,
                hair_color: row.2,
            });
            Ok(())
        })
        .unwrap();
        users
    })
}

pub fn bench_trivial_query_by_name(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let stmt = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let mut users = Vec::new();
        conn.exec_foreach(&stmt, (), |user: User| {
            users.push(user);
            Ok(())
        })
        .unwrap();
        users
    })
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    });

    let stmt = conn.prepare(MEDIUM_COMPLEX_QUERY_BY_ID).unwrap();

    b.iter(|| {
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
        .unwrap();
        results
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let mut conn = connection();

    b.iter(|| insert_users(size, &mut conn, |_| Some("hair_color")))
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    let mut conn = connection();

    insert_users(100, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    });

    // Get user IDs
    let user_ids: Vec<(i32,)> = conn.query_collect("SELECT id FROM users").unwrap();

    // Insert posts - build values directly in SQL for simplicity
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
    conn.query_drop(&insert_query).unwrap();

    // Get post IDs
    let all_posts: Vec<(i32,)> = conn.query_collect("SELECT id FROM posts").unwrap();

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
    conn.query_drop(&insert_query).unwrap();

    b.iter(|| {
        // Load users
        let users: Vec<User> = conn
            .query_collect("SELECT id, name, hair_color FROM users")
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
        let posts: Vec<Post> = conn.query_collect(&posts_query).unwrap();

        let post_ids_str = posts
            .iter()
            .map(|p| p.id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let comments_query = format!(
            "SELECT id, post_id, text FROM comments WHERE post_id IN({})",
            post_ids_str
        );
        let comments: Vec<Comment> = conn.query_collect(&comments_query).unwrap();

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
}
