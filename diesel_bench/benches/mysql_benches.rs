use super::consts::mysql::{
    CLEANUP_QUERIES, MEDIUM_COMPLEX_QUERY_BY_ID, MEDIUM_COMPLEX_QUERY_BY_NAME,
};
use super::Bencher;
use rust_mysql::params::Params;
use rust_mysql::prelude::*;
use rust_mysql::{Conn, Opts, OptsBuilder, Row};
use std::collections::HashMap;

pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

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
    let opts =
        OptsBuilder::from_opts(Opts::from_url(&connection_url).unwrap()).prefer_socket(false);
    let mut conn = Conn::new(opts).unwrap();

    for query in CLEANUP_QUERIES {
        conn.query_drop(query).unwrap();
    }

    conn
}

fn insert_users(size: usize, conn: &mut Conn, hair_color_init: impl Fn(usize) -> Option<String>) {
    let mut query = String::from("INSERT INTO users (name, hair_color) VALUES");

    let mut params = Vec::with_capacity(2 * size);

    for x in 0..size {
        query += &format!("{} (?, ?)", if x == 0 { "" } else { "," },);
        params.push((format!("User {}", x), hair_color_init(x)));
    }

    let query = conn.prep(&query).unwrap();

    let params = params
        .iter()
        .flat_map(|(a, b)| vec![a as &dyn ToValue, b as _])
        .collect::<Vec<_>>();
    let params: &[&dyn ToValue] = &params;
    let params: Params = params.into();

    conn.exec_drop(query, params).unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let query = conn.prep("SELECT id, name, hair_color FROM users").unwrap();

    b.iter(|| {
        conn.exec_map(&query, Params::Empty, |mut row: Row| User {
            id: row.take(0).unwrap(),
            name: row.take(1).unwrap(),
            hair_color: row.take(2).unwrap(),
        })
        .unwrap()
    })
}

pub fn bench_trivial_query_by_name(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let query = conn.prep("SELECT id, name, hair_color FROM users").unwrap();

    b.iter(|| {
        conn.exec_map(&query, Params::Empty, |mut row: Row| User {
            id: row.take("id").unwrap(),
            name: row.take("name").unwrap(),
            hair_color: row.take("hair_color").unwrap(),
        })
        .unwrap()
    })
}

pub fn bench_medium_complex_query_by_id(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let query = conn.prep(MEDIUM_COMPLEX_QUERY_BY_ID).unwrap();

    b.iter(|| {
        conn.exec_map(
            &query,
            Params::Positional(vec!["black".into()]),
            |mut row: Row| {
                let user = User {
                    id: row.take(0).unwrap(),
                    name: row.take(1).unwrap(),
                    hair_color: row.take(2).unwrap(),
                };
                let post = if let Some(id) = row.take(3).unwrap() {
                    Some(Post {
                        id,
                        user_id: row.take(4).unwrap(),
                        title: row.take(5).unwrap(),
                        body: row.take(6).unwrap(),
                    })
                } else {
                    None
                };
                (user, post)
            },
        )
        .unwrap()
    })
}

pub fn bench_medium_complex_query_by_name(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let query = conn.prep(MEDIUM_COMPLEX_QUERY_BY_NAME).unwrap();

    b.iter(|| {
        conn.exec_map(
            &query,
            Params::Positional(vec!["black".into()]),
            |mut row: Row| {
                let user = User {
                    id: row.take("myuser_id").unwrap(),
                    name: row.take("name").unwrap(),
                    hair_color: row.take("hair_color").unwrap(),
                };
                let post = if let Some(id) = row.take("post_id").unwrap() {
                    Some(Post {
                        id,
                        user_id: row.take("user_id").unwrap(),
                        title: row.take("title").unwrap(),
                        body: row.take("body").unwrap(),
                    })
                } else {
                    None
                };
                (user, post)
            },
        )
        .unwrap()
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let mut client = connection();

    b.iter(|| insert_users(size, &mut client, |_| Some(String::from("hair_color"))))
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    let mut client = connection();

    insert_users(100, &mut client, |i| {
        Some(if i % 2 == 0 {
            String::from("black")
        } else {
            String::from("brown")
        })
    });

    let user_ids = client
        .exec_map("SELECT id FROM users", Params::Empty, |mut row: Row| {
            row.take(0).unwrap()
        })
        .unwrap();

    let data = user_ids
        .iter()
        .flat_map(|user_id: &i32| {
            (0..10).map(move |i| (format!("Post {} by user {}", i, user_id), user_id, None))
        })
        .collect::<Vec<_>>();

    let mut insert_query = String::from("INSERT INTO posts(title, user_id, body) VALUES");

    for x in 0..data.len() {
        insert_query += &format!("{} (?, ?, ?)", if x == 0 { "" } else { "," },);
    }

    let data = data
        .iter()
        .flat_map(|(title, user_id, body): &(_, _, Option<String>)| {
            vec![title as &(dyn ToValue), user_id as _, body as _]
        })
        .collect::<Vec<_>>();

    client
        .exec_drop(&insert_query as &str, &data as &[&dyn ToValue])
        .unwrap();

    let all_posts = client
        .exec_map("SELECT id FROM posts", Params::Empty, |mut row: Row| {
            row.take(0).unwrap()
        })
        .unwrap();

    let data = all_posts
        .iter()
        .flat_map(|post_id: &i32| {
            (0..10).map(move |i| (format!("Comment {} on post {}", i, post_id), post_id))
        })
        .collect::<Vec<_>>();

    let mut insert_query = String::from("INSERT INTO comments(text, post_id) VALUES");

    for x in 0..data.len() {
        insert_query += &format!("{} (?, ?)", if x == 0 { "" } else { "," },);
    }

    let data = data
        .iter()
        .flat_map(|(title, post_id)| vec![title as &dyn ToValue, post_id as _])
        .collect::<Vec<_>>();

    client
        .exec_drop(&insert_query as &str, &data as &[&dyn ToValue])
        .unwrap();

    let user_query = client
        .prep("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let users = client
            .exec_map(&user_query, Params::Empty, |mut row: Row| User {
                id: row.take("id").unwrap(),
                name: row.take("name").unwrap(),
                hair_color: row.take("hair_color").unwrap(),
            })
            .unwrap();

        let mut posts_query =
            String::from("SELECT id, title, user_id, body FROM posts WHERE user_id IN(");

        let user_ids = users
            .iter()
            .enumerate()
            .map(|(i, &User { ref id, .. })| {
                posts_query += &format!("{}?", if i == 0 { "" } else { "," });
                id as &dyn ToValue
            })
            .collect::<Vec<_>>();

        posts_query += ")";

        let posts = client
            .exec_map(
                &posts_query,
                &user_ids as &[&dyn ToValue],
                |mut row: Row| Post {
                    id: row.take("id").unwrap(),
                    title: row.take("title").unwrap(),
                    user_id: row.take("user_id").unwrap(),
                    body: row.take("body").unwrap(),
                },
            )
            .unwrap();

        let mut comments_query =
            String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");

        let post_ids = posts
            .iter()
            .enumerate()
            .map(|(i, &Post { ref id, .. })| {
                comments_query += &format!("{}?", if i == 0 { "" } else { "," });
                id as &dyn ToValue
            })
            .collect::<Vec<_>>();

        comments_query += ")";

        let comments = client
            .exec_map(
                &comments_query,
                &post_ids as &[&dyn ToValue],
                |mut row: Row| Comment {
                    id: row.take("id").unwrap(),
                    post_id: row.take("post_id").unwrap(),
                    text: row.take("text").unwrap(),
                },
            )
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
