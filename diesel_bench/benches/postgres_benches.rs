use super::consts::postgres::{
    CLEANUP_QUERIES, MEDIUM_COMPLEX_QUERY_BY_ID, MEDIUM_COMPLEX_QUERY_BY_NAME,
};
use super::Bencher;
use rust_postgres::fallible_iterator::FallibleIterator;
use rust_postgres::types::ToSql;
use rust_postgres::{Client, NoTls};
use std::collections::HashMap;

const NO_PARAMS: Vec<&dyn ToSql> = Vec::new();

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

fn connection() -> Client {
    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("POSTGRES_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut client = Client::connect(&connection_url, NoTls).unwrap();

    for query in CLEANUP_QUERIES {
        client.execute(*query, &[]).unwrap();
    }

    client
}

fn insert_users(size: usize, conn: &mut Client, hair_color_init: impl Fn(usize) -> Option<String>) {
    let mut query = String::from("INSERT INTO users (name, hair_color) VALUES");

    let mut params = Vec::with_capacity(2 * size);

    for x in 0..size {
        query += &format!(
            "{} (${}, ${})",
            if x == 0 { "" } else { "," },
            2 * x + 1,
            2 * x + 2
        );
        params.push((format!("User {}", x), hair_color_init(x)));
    }

    let params = params
        .iter()
        .flat_map(|(a, b)| vec![a as _, b as _])
        .collect::<Vec<_>>();

    conn.execute(&query as &str, &params).unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let mut client = connection();
    insert_users(size, &mut client, |_| None);

    let query = client
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        client
            .query_raw(&query, NO_PARAMS)
            .unwrap()
            .map(|row| {
                Ok(User {
                    id: row.get(0),
                    name: row.get(1),
                    hair_color: row.get(2),
                })
            })
            .collect::<Vec<_>>()
            .unwrap()
    })
}

pub fn bench_trivial_query_by_name(b: &mut Bencher, size: usize) {
    let mut client = connection();
    insert_users(size, &mut client, |_| None);

    let query = client
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        client
            .query_raw(&query, NO_PARAMS)
            .unwrap()
            .map(|row| {
                Ok(User {
                    id: row.get("id"),
                    name: row.get("name"),
                    hair_color: row.get("hair_color"),
                })
            })
            .collect::<Vec<_>>()
            .unwrap()
    })
}

pub fn bench_medium_complex_query_by_id(b: &mut Bencher, size: usize) {
    let mut client = connection();
    insert_users(size, &mut client, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let query = client.prepare(MEDIUM_COMPLEX_QUERY_BY_ID).unwrap();

    b.iter(|| {
        client
            .query_raw(&query, &[&"black"])
            .unwrap()
            .map(|row| {
                let user = User {
                    id: row.get(0),
                    name: row.get(1),
                    hair_color: row.get(2),
                };
                let post = if let Some(id) = row.get(3) {
                    Some(Post {
                        id,
                        user_id: row.get(4),
                        title: row.get(5),
                        body: row.get(6),
                    })
                } else {
                    None
                };
                Ok((user, post))
            })
            .collect::<Vec<_>>()
            .unwrap()
    })
}

pub fn bench_medium_complex_query_by_name(b: &mut Bencher, size: usize) {
    let mut client = connection();
    insert_users(size, &mut client, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let query = client.prepare(MEDIUM_COMPLEX_QUERY_BY_NAME).unwrap();

    b.iter(|| {
        client
            .query_raw(&query, &[&"black"])
            .unwrap()
            .map(|row| {
                let user = User {
                    id: row.get("myuser_id"),
                    name: row.get("name"),
                    hair_color: row.get("hair_color"),
                };
                let post = if let Some(id) = row.get("post_id") {
                    Some(Post {
                        id,
                        user_id: row.get("user_id"),
                        title: row.get("title"),
                        body: row.get("body"),
                    })
                } else {
                    None
                };
                Ok((user, post))
            })
            .collect::<Vec<_>>()
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
        .query_raw("SELECT id FROM users", NO_PARAMS)
        .unwrap()
        .map(|row| Ok(row.get("id")))
        .collect::<Vec<i32>>()
        .unwrap();

    let data = user_ids
        .iter()
        .flat_map(|user_id| {
            (0..10).map(move |i| (format!("Post {} by user {}", i, user_id), user_id, None))
        })
        .collect::<Vec<_>>();

    let mut insert_query = String::from("INSERT INTO posts(title, user_id, body) VALUES");

    for x in 0..data.len() {
        insert_query += &format!(
            "{} (${}, ${}, ${})",
            if x == 0 { "" } else { "," },
            3 * x + 1,
            3 * x + 2,
            3 * x + 3
        );
    }

    let data = data
        .iter()
        .flat_map(|(title, user_id, body): &(_, _, Option<String>)| {
            vec![title as &(dyn ToSql + Sync), user_id as _, body as _]
        })
        .collect::<Vec<_>>();

    client.execute(&insert_query as &str, &data).unwrap();

    let all_posts = client
        .query_raw("SELECT id FROM posts", NO_PARAMS)
        .unwrap()
        .map(|row| Ok(row.get("id")))
        .collect::<Vec<i32>>()
        .unwrap();

    let data = all_posts
        .iter()
        .flat_map(|post_id| {
            (0..10).map(move |i| (format!("Comment {} on post {}", i, post_id), post_id))
        })
        .collect::<Vec<_>>();

    let mut insert_query = String::from("INSERT INTO comments(text, post_id) VALUES");

    for x in 0..data.len() {
        insert_query += &format!(
            "{} (${}, ${})",
            if x == 0 { "" } else { "," },
            2 * x + 1,
            2 * x + 2,
        );
    }

    let data = data
        .iter()
        .flat_map(|(title, post_id)| vec![title as _, post_id as _])
        .collect::<Vec<_>>();

    client.execute(&insert_query as &str, &data).unwrap();

    let user_query = client
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let users = client
            .query_raw(&user_query, NO_PARAMS)
            .unwrap()
            .map(|row| {
                Ok(User {
                    id: row.get("id"),
                    name: row.get("name"),
                    hair_color: row.get("hair_color"),
                })
            })
            .collect::<Vec<_>>()
            .unwrap();

        let mut posts_query =
            String::from("SELECT id, title, user_id, body FROM posts WHERE user_id IN(");

        let user_ids = users
            .iter()
            .enumerate()
            .map(|(i, &User { id, .. })| {
                posts_query += &format!("{}${}", if i == 0 { "" } else { "," }, i + 1);
                id
            })
            .collect::<Vec<i32>>();

        posts_query += ")";

        let posts = client
            .query_raw(&posts_query as &str, user_ids)
            .unwrap()
            .map(|row| {
                Ok(Post {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                })
            })
            .collect::<Vec<_>>()
            .unwrap();

        let mut comments_query =
            String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");

        let post_ids = posts
            .iter()
            .enumerate()
            .map(|(i, &Post { id, .. })| {
                comments_query += &format!("{}${}", if i == 0 { "" } else { "," }, i + 1);
                id
            })
            .collect::<Vec<i32>>();

        comments_query += ")";

        let comments = client
            .query_raw(&comments_query as &str, post_ids)
            .unwrap()
            .map(|row| {
                Ok(Comment {
                    id: row.get("id"),
                    post_id: row.get("post_id"),
                    text: row.get("text"),
                })
            })
            .collect::<Vec<_>>()
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
