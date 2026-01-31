use crate::consts::mysql::{CLEANUP_QUERIES, MEDIUM_COMPLEX_QUERY_BY_ID};
use crate::Bencher;
use std::collections::HashMap;
use std::fmt::Write;
use zero_mysql::r#macro::FromRawRow;
use zero_mysql::sync::Conn;
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

fn connection() -> Conn {
    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("MYSQL_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut opts = Opts::try_from(connection_url.as_str()).unwrap();
    opts.upgrade_to_unix_socket = false;
    let mut conn: Conn = Conn::new(opts).unwrap();

    conn.query_drop(&CLEANUP_QUERIES.join("; ")).unwrap();

    conn
}

fn insert_users(
    size: usize,
    conn: &mut Conn,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let mut query = String::from("INSERT INTO users (name, hair_color) VALUES ");
    let mut params: Vec<zero_mysql::Value> = Vec::with_capacity(size * 2);
    for x in 0..size {
        if x > 0 {
            query.push(',');
        }
        query.push_str("(?, ?)");
        params.push(format!("User {}", x).into());
        params.push(hair_color_init(x).into());
    }
    let mut stmt = conn.prepare(&query).unwrap();
    conn.exec_drop(&mut stmt, params).unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let mut stmt = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let mut users = Vec::new();
        conn.exec_foreach(&mut stmt, (), |row: (i32, String, Option<String>)| {
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

    let mut stmt = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let mut users = Vec::new();
        conn.exec_foreach(&mut stmt, (), |user: User| {
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

    let mut stmt = conn.prepare(MEDIUM_COMPLEX_QUERY_BY_ID).unwrap();

    b.iter(|| {
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

    let mut user_ids = Vec::new();
    let mut stmt = conn.prepare("SELECT id FROM users").unwrap();
    conn.exec_foreach(&mut stmt, (), |row: (i32,)| {
        user_ids.push(row.0);
        Ok(())
    })
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
    conn.query_drop(&insert_query).unwrap();

    let mut all_posts = Vec::new();
    let mut stmt = conn.prepare("SELECT id FROM posts").unwrap();
    conn.exec_foreach(&mut stmt, (), |row: (i32,)| {
        all_posts.push(row.0);
        Ok(())
    })
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
    conn.query_drop(&insert_query).unwrap();

    let mut user_stmt = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let mut users = Vec::new();
        conn.exec_foreach(&mut user_stmt, (), |user: User| {
            users.push(user);
            Ok(())
        })
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
        let mut stmt = conn.prepare(&posts_query).unwrap();
        conn.exec_foreach(&mut stmt, (), |post: Post| {
            posts.push(post);
            Ok(())
        })
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
        let mut stmt = conn.prepare(&comments_query).unwrap();
        conn.exec_foreach(&mut stmt, (), |comment: Comment| {
            comments.push(comment);
            Ok(())
        })
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
}
