use super::consts;
use super::Bencher;
use std::{collections::HashMap, fmt::Write};
use tokio::{net::TcpStream, runtime::Runtime};
use wtx::{
    database::{Executor, Record},
    misc::{Either, UriRef, Wrapper},
    rng::{ChaCha20, SeedableRng},
};

#[cfg(feature = "mysql")]
use wtx::database::client::mysql::{Config, ExecutorBuffer, MysqlExecutor as LocalExecutor};
#[cfg(feature = "postgres")]
use wtx::database::client::postgres::{Config, ExecutorBuffer, PostgresExecutor as LocalExecutor};

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
            conn.execute_stmt_many(stmt_hash, (), |record| {
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
            concat(0..users.len(), &mut posts_query, |local_str, _idx| {
                #[cfg(feature = "postgres")]
                local_str.write_fmt(format_args!("${}", _idx + 1)).unwrap();
                #[cfg(feature = "mysql")]
                local_str.push('?');
            });
            posts_query.push(')');

            let mut posts = Vec::with_capacity(LEN);
            conn.execute_stmt_many(
                posts_query.as_str(),
                Wrapper(users.iter().map(|user| user.id)),
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
            assert_eq!(posts.len(), LEN * 10);

            let mut comments_query =
                String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");
            concat(0..posts.len(), &mut comments_query, |local_str, _idx| {
                #[cfg(feature = "postgres")]
                local_str.write_fmt(format_args!("${}", _idx + 1)).unwrap();
                #[cfg(feature = "mysql")]
                local_str.push('?');
            });
            comments_query.push(')');

            let mut comments = Vec::with_capacity(LEN);
            conn.execute_stmt_many(
                comments_query.as_str(),
                Wrapper(posts.iter().map(|post| post.id)),
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
            assert_eq!(comments.len(), LEN * 10 * 10);

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
        #[cfg(feature = "postgres")]
        let stmt_hash = conn
            .prepare(consts::postgres::MEDIUM_COMPLEX_QUERY_BY_ID)
            .await
            .unwrap();
        #[cfg(feature = "mysql")]
        let stmt_hash = conn
            .prepare(consts::mysql::MEDIUM_COMPLEX_QUERY_BY_ID)
            .await
            .unwrap();
        (conn, stmt_hash)
    });
    b.iter(|| {
        runtime.block_on(async {
            let mut _rslt = Vec::with_capacity(size);
            conn.execute_stmt_many(stmt_hash, ("black",), |record| {
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
        runtime.block_on(async {
            let mut users = Vec::with_capacity(size);
            conn.execute_stmt_many(stmt_hash, (), |record| {
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

async fn connection() -> LocalExecutor<wtx::Error, ExecutorBuffer, TcpStream> {
    dotenvy::dotenv().ok();
    let url = dotenvy::var("POSTGRES_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let uri = UriRef::new(url.as_str());
    let mut rng = ChaCha20::from_os().unwrap();
    let stream = TcpStream::connect(uri.hostname_with_implied_port())
        .await
        .unwrap();
    stream.set_nodelay(true).unwrap();
    #[cfg(feature = "postgres")]
    let buffer = ExecutorBuffer::with_capacity((512, 8192, 512, 32), 32, &mut rng).unwrap();
    #[cfg(feature = "mysql")]
    let buffer = ExecutorBuffer::with_capacity((512, 512, 8192, 512, 32), 32, &mut rng).unwrap();
    let mut conn =
        LocalExecutor::connect(&Config::from_uri(&uri).unwrap(), buffer, &mut rng, stream)
            .await
            .unwrap();
    #[cfg(feature = "postgres")]
    conn.execute_ignored(&consts::postgres::CLEANUP_QUERIES.join(";"))
        .await
        .unwrap();
    #[cfg(feature = "mysql")]
    conn.execute_ignored(&consts::mysql::CLEANUP_QUERIES.join(";"))
        .await
        .unwrap();
    conn
}

async fn insert_posts<const N: usize>(
    conn: &mut LocalExecutor<wtx::Error, ExecutorBuffer, TcpStream>,
) {
    let mut users_ids: Vec<i32> = Vec::with_capacity(N);
    conn.execute_stmt_many("SELECT id FROM users", (), |record| {
        users_ids.push(record.decode(0).unwrap());
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(users_ids.len(), N);

    let params = users_ids
        .into_iter()
        .flat_map(|user_id| {
            (0..10).map(move |idx| {
                [
                    Either::Left(format!("Post {idx} by user {user_id}")),
                    Either::Right(Either::Left(user_id)),
                    Either::Right(Either::Right(None as Option<&'static str>)),
                ]
            })
        })
        .collect::<Vec<_>>();

    let mut insert_stmt = String::from("INSERT INTO posts(title, user_id, body) VALUES ");
    concat(
        0..params.len(),
        &mut insert_stmt,
        |local_insert_stmt, _idx| {
            #[cfg(feature = "postgres")]
            local_insert_stmt
                .write_fmt(format_args!(
                    "(${}, ${}, ${})",
                    3 * _idx + 1,
                    3 * _idx + 2,
                    3 * _idx + 3
                ))
                .unwrap();
            #[cfg(feature = "mysql")]
            local_insert_stmt.push_str("(?, ?, ?)");
        },
    );

    conn.execute_stmt_ignored(insert_stmt.as_str(), Wrapper(params.into_iter().flatten()))
        .await
        .unwrap();

    let mut post_ids: Vec<i32> = Vec::with_capacity(N * 10);
    conn.execute_stmt_many("SELECT id FROM posts", (), |record| {
        post_ids.push(record.decode(0).unwrap());
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(post_ids.len(), N * 10);

    let params = post_ids
        .into_iter()
        .flat_map(|post_id| {
            (0..10).map(move |idx| {
                [
                    Either::Left(format!("Comment {idx} for post {post_id}")),
                    Either::Right(post_id),
                ]
            })
        })
        .collect::<Vec<_>>();

    let mut insert_stmt = String::from("INSERT INTO comments(text, post_id) VALUES ");
    concat(
        0..params.len(),
        &mut insert_stmt,
        |local_insert_stmt, _idx| {
            #[cfg(feature = "postgres")]
            local_insert_stmt
                .write_fmt(format_args!("(${}, ${})", 2 * _idx + 1, 2 * _idx + 2,))
                .unwrap();
            #[cfg(feature = "mysql")]
            local_insert_stmt.push_str("(?, ?)");
        },
    );

    conn.execute_stmt_ignored(insert_stmt.as_str(), Wrapper(params.into_iter().flatten()))
        .await
        .unwrap();

    let mut count: Vec<i64> = Vec::with_capacity(N * 10);
    conn.execute_stmt_many("SELECT count(id) FROM comments", (), |record| {
        count.push(record.decode(0).unwrap());
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(count[0] as usize, N * 10 * 10);
}

async fn insert_users<const N: usize>(
    conn: &mut LocalExecutor<wtx::Error, ExecutorBuffer, TcpStream>,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let mut query = String::from("INSERT INTO users (name, hair_color) VALUES ");
    concat(0..N, &mut query, |local_query, _idx| {
        #[cfg(feature = "postgres")]
        local_query
            .write_fmt(format_args!("(${}, ${})", 2 * _idx + 1, 2 * _idx + 2))
            .unwrap();
        #[cfg(feature = "mysql")]
        local_query.push_str("(?, ?)");
    });

    let params = (0..N).into_iter().flat_map(|idx| {
        [
            Either::Left(format!("User {idx}")),
            Either::Right(hair_color_init(idx)),
        ]
    });

    conn.execute_stmt_ignored(query.as_str(), Wrapper(params))
        .await
        .unwrap();
}
