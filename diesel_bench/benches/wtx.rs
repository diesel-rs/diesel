use crate::Bencher;
use std::collections::HashMap;
use std::fmt::Write;
use tokio::{net::TcpStream, runtime::Runtime};
use wtx::{
    database::{
        client::postgres::{Config, Executor, ExecutorBuffer},
        Encode, Executor as _, Record as _,
    },
    misc::UriPartsRef,
    rng::StdRng,
};

pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub text: String,
}

pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().expect("Failed to create runtime");
    let mut conn = runtime.block_on(connection());
    fn hair_color_callback(_: usize) -> Option<&'static str> {
        Some("hair_color")
    }
    match size {
        1 => b.iter(|| runtime.block_on(insert_users::<1>(&mut conn, hair_color_callback))),
        10 => b.iter(|| runtime.block_on(insert_users::<10>(&mut conn, hair_color_callback))),
        25 => b.iter(|| runtime.block_on(insert_users::<25>(&mut conn, hair_color_callback))),
        50 => b.iter(|| runtime.block_on(insert_users::<50>(&mut conn, hair_color_callback))),
        100 => b.iter(|| runtime.block_on(insert_users::<100>(&mut conn, hair_color_callback))),
        _ => unimplemented!(),
    }
}

pub fn bench_loading_associations_sequentially(b: &mut Bencher) {
    const LEN: usize = 100;

    let runtime = Runtime::new().expect("Failed to create runtime");

    let (mut conn, stmt_hash) = runtime.block_on(async {
        let mut conn = connection().await;

        insert_users::<LEN>(&mut conn, |i| {
            Some(if i % 2 == 0 { "black" } else { "brown" })
        })
        .await;

        insert_posts::<LEN>(&mut conn).await;

        let stmt_hash = conn
            .prepare("SELECT id, name, hair_color FROM users")
            .await
            .unwrap();

        (conn, stmt_hash)
    });

    b.iter(|| {
        runtime.block_on(async {
            let mut users = Vec::with_capacity(LEN);
            conn.fetch_many_with_stmt(stmt_hash, (), |record| {
                users.push(User {
                    hair_color: record.decode_opt(2).unwrap(),
                    id: record.decode(0).unwrap(),
                    name: record.decode(1).unwrap(),
                });
                Ok(())
            })
            .await
            .unwrap();

            let mut posts_query =
                String::from("SELECT id, title, user_id, body FROM posts WHERE user_id IN(");
            let mut users_ids = Vec::with_capacity(LEN);
            concat(
                users.iter().enumerate(),
                &mut posts_query,
                |local_str, (idx, &User { id, .. })| {
                    local_str.write_fmt(format_args!("${}", idx + 1)).unwrap();
                    users_ids.push(id);
                },
            );
            posts_query += ")";

            let mut posts = Vec::with_capacity(LEN);
            conn.fetch_many_with_stmt::<wtx::Error, _, _>(
                posts_query.as_str(),
                users_ids.as_slice(),
                |record| {
                    posts.push(Post {
                        body: record.decode_opt(3).unwrap(),
                        id: record.decode(0).unwrap(),
                        title: record.decode(1).unwrap(),
                        user_id: record.decode(2).unwrap(),
                    });
                    Ok(())
                },
            )
            .await
            .unwrap();

            let mut comments_query =
                String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");
            let mut posts_ids = Vec::with_capacity(LEN);
            concat(
                posts.iter().enumerate(),
                &mut comments_query,
                |local_str, (idx, &Post { id, .. })| {
                    local_str.write_fmt(format_args!("${}", idx + 1)).unwrap();
                    posts_ids.push(id);
                },
            );
            comments_query += ")";

            let mut comments = Vec::with_capacity(LEN);
            conn.fetch_many_with_stmt::<wtx::Error, _, _>(
                comments_query.as_str(),
                posts_ids.as_slice(),
                |record| {
                    comments.push(Comment {
                        id: record.decode(0).unwrap(),
                        post_id: record.decode(1).unwrap(),
                        text: record.decode(2).unwrap(),
                    });
                    Ok(())
                },
            )
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

            let _ = users
                .into_iter()
                .map(|(_, users_with_post_and_comment)| users_with_post_and_comment)
                .collect::<Vec<(User, Vec<(Post, Vec<Comment>)>)>>();
        });
    })
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().expect("Failed to create runtime");
    let (mut conn, stmt_hash) = runtime.block_on(async {
        let mut conn = connection().await;
        let hair_color_callback = |i| Some(if i % 2 == 0 { "black" } else { "brown" });
        match size {
            1 => insert_users::<1>(&mut conn, hair_color_callback).await,
            10 => insert_users::<10>(&mut conn, hair_color_callback).await,
            100 => insert_users::<100>(&mut conn, hair_color_callback).await,
            1_000 => insert_users::<1_000>(&mut conn, hair_color_callback).await,
            10_000 => insert_users::<10_000>(&mut conn, hair_color_callback).await,
            _ => unimplemented!(),
        }
        let stmt_hash = conn
            .prepare(
                "SELECT u.id, u.name, u.hair_color, p.id, p.user_id, p.title, p.body \
                FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = $1",
            )
            .await
            .unwrap();
        (conn, stmt_hash)
    });
    b.iter(|| {
        runtime.block_on(async {
            let mut _rslt = Vec::with_capacity(size);
            conn.fetch_many_with_stmt::<wtx::Error, _, _>(stmt_hash, ("black",), |record| {
                let user = User {
                    id: record.decode(0).unwrap(),
                    name: record.decode(1).unwrap(),
                    hair_color: record.decode_opt(2).unwrap(),
                };
                let post = record.decode_opt::<_, i32>(3).unwrap().map(|id| Post {
                    id,
                    user_id: record.decode(4).unwrap(),
                    title: record.decode(5).unwrap(),
                    body: record.decode_opt(6).unwrap(),
                });
                _rslt.push((user, post));
                Ok(())
            })
            .await
            .unwrap();
        });
    })
}

pub fn bench_trivial_query(b: &mut Bencher, size: usize) {
    let runtime = Runtime::new().expect("Failed to create runtime");
    let (mut conn, stmt_hash) = runtime.block_on(async {
        let mut conn = connection().await;
        match size {
            1 => insert_users::<1>(&mut conn, |_| None).await,
            10 => insert_users::<10>(&mut conn, |_| None).await,
            100 => insert_users::<100>(&mut conn, |_| None).await,
            1_000 => insert_users::<1_000>(&mut conn, |_| None).await,
            10_000 => insert_users::<10_000>(&mut conn, |_| None).await,
            _ => unimplemented!(),
        };
        let stmt_hash = conn
            .prepare("SELECT id, name, hair_color FROM users")
            .await
            .unwrap();
        (conn, stmt_hash)
    });
    b.iter(|| {
        let mut users = Vec::with_capacity(size);
        runtime.block_on(async {
            conn.fetch_many_with_stmt(stmt_hash, (), |record| {
                users.push(User {
                    id: record.decode(0).unwrap(),
                    name: record.decode(1).unwrap(),
                    hair_color: record.decode_opt(2).unwrap(),
                });
                Ok(())
            })
            .await
            .unwrap();
        })
    })
}

fn concat<I>(mut iter: I, string: &mut String, mut cb: impl FnMut(&mut String, I::Item))
where
    I: Iterator,
{
    if let Some(elem) = iter.next() {
        cb(string, elem);
    }
    for elem in iter {
        string.push(',');
        cb(string, elem);
    }
}

async fn connection() -> Executor<ExecutorBuffer, TcpStream> {
    dotenvy::dotenv().ok();
    let url = dotenvy::var("POSTGRES_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let up = UriPartsRef::new(url.as_str());
    let mut rng = StdRng::default();
    let mut conn = Executor::connect(
        &Config::from_uri_parts(&up).unwrap(),
        ExecutorBuffer::with_default_params(&mut rng),
        &mut rng,
        TcpStream::connect(up.host()).await.unwrap(),
    )
    .await
    .unwrap();
    conn.execute("TRUNCATE TABLE comments CASCADE", |_| {})
        .await
        .unwrap();
    conn.execute("TRUNCATE TABLE posts CASCADE", |_| {})
        .await
        .unwrap();
    conn.execute("TRUNCATE TABLE users CASCADE", |_| {})
        .await
        .unwrap();
    conn
}

async fn insert_posts<const N: usize>(conn: &mut Executor<ExecutorBuffer, TcpStream>) {
    let mut users_ids = Vec::with_capacity(N);
    conn.fetch_many_with_stmt("SELECT id FROM users", (), |record| {
        users_ids.push(record.decode(0).unwrap());
        Ok(())
    })
    .await
    .unwrap();

    let params = users_ids
        .into_iter()
        .flat_map(|user_id| {
            (0..10).map(move |idx| (format!("Post {idx} by user {user_id}"), user_id, None))
        })
        .collect::<Vec<_>>();

    let mut insert_stmt = String::from("INSERT INTO posts(title, user_id, body) VALUES");
    concat(
        0..params.len(),
        &mut insert_stmt,
        |local_insert_stmt, idx| {
            local_insert_stmt
                .write_fmt(format_args!(
                    "(${}, ${}, ${})",
                    3 * idx + 1,
                    3 * idx + 2,
                    3 * idx + 3
                ))
                .unwrap();
        },
    );

    let params_ref = params
        .iter()
        .flat_map(|el: &(String, i32, Option<String>)| {
            let a: &dyn Encode<_, _> = &el.0;
            let b: &dyn Encode<_, _> = &el.1;
            let c: &dyn Encode<_, _> = &el.2;
            [a, b, c]
        })
        .collect::<Vec<&dyn Encode<_, _>>>();

    conn.execute_with_stmt::<wtx::Error, _, _>(insert_stmt.as_str(), params_ref.as_slice())
        .await
        .unwrap();
}

async fn insert_users<const N: usize>(
    conn: &mut Executor<ExecutorBuffer, TcpStream>,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let mut query = String::from("INSERT INTO users (name, hair_color) VALUES");
    let mut params = Vec::with_capacity(2 * N);

    concat(0..N, &mut query, |local_query, idx| {
        local_query
            .write_fmt(format_args!("(${}, ${})", 2 * idx + 1, 2 * idx + 2))
            .unwrap();
        params.push((format!("User {idx}"), hair_color_init(idx)));
    });

    let params_ref = params
        .iter()
        .flat_map(|el: &(String, Option<&'static str>)| {
            let a: &dyn Encode<_, _> = &el.0;
            let b: &dyn Encode<_, _> = &el.1;
            [a, b]
        })
        .collect::<Vec<_>>();

    conn.execute_with_stmt::<wtx::Error, _, _>(query.as_str(), params_ref.as_slice())
        .await
        .unwrap();
}
