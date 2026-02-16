use super::consts;
use super::Bencher;
use sqlx::*;
use std::collections::HashMap;
use tokio::runtime::Runtime;

#[derive(sqlx::FromRow)]
pub struct User {
    #[cfg(not(feature = "sqlite"))]
    pub id: i32,
    #[cfg(feature = "sqlite")]
    pub id: i64,
    pub name: String,
    pub hair_color: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct Post {
    #[cfg(not(feature = "sqlite"))]
    pub id: i32,
    #[cfg(feature = "sqlite")]
    pub id: i64,
    #[cfg(not(feature = "sqlite"))]
    pub user_id: i32,
    #[cfg(feature = "sqlite")]
    pub user_id: i64,
    pub title: String,
    pub body: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct UserWithPost {
    #[cfg(not(feature = "sqlite"))]
    pub myuser_id: i32,
    #[cfg(feature = "sqlite")]
    pub myuser_id: i64,
    pub name: String,
    pub hair_color: Option<String>,
    #[cfg(not(feature = "sqlite"))]
    pub post_id: Option<i32>,
    #[cfg(feature = "sqlite")]
    pub post_id: Option<i64>,
    #[cfg(not(feature = "sqlite"))]
    pub user_id: Option<i32>,
    #[cfg(feature = "sqlite")]
    pub user_id: Option<i64>,
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct Comment {
    pub id: i32,
    #[cfg(feature = "sqlite")]
    pub post_id: i64,
    #[cfg(not(feature = "sqlite"))]
    pub post_id: i32,
    pub text: String,
}

#[cfg(feature = "sqlite")]
type Connection = sqlx::SqliteConnection;

#[cfg(feature = "mysql")]
type Connection = sqlx::MySqlConnection;

#[cfg(feature = "postgres")]
type Connection = sqlx::PgConnection;

#[cfg(feature = "postgres")]
fn connection() -> (Runtime, Connection) {
    use sqlx::Connection;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("POSTGRES_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");

    let conn = runtime.block_on(async {
        let mut conn = sqlx::PgConnection::connect(&connection_url).await.unwrap();
        for query in consts::postgres::CLEANUP_QUERIES {
            sqlx::query(query).execute(&mut conn).await.unwrap();
        }
        conn
    });

    (runtime, conn)
}

#[cfg(feature = "mysql")]
fn connection() -> (Runtime, Connection) {
    use sqlx::Connection;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("MYSQL_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");

    let conn = runtime.block_on(async {
        let mut conn = sqlx::MySqlConnection::connect(&connection_url)
            .await
            .unwrap();
        for query in consts::mysql::CLEANUP_QUERIES {
            sqlx::query(query).execute(&mut conn).await.unwrap();
        }
        conn
    });

    (runtime, conn)
}

#[cfg(feature = "sqlite")]
fn connection() -> (Runtime, Connection) {
    use sqlx::Connection;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    let conn = runtime.block_on(async {
        let mut conn = sqlx::SqliteConnection::connect("sqlite::memory:")
            .await
            .unwrap();

        for migration in super::SQLITE_MIGRATION_SQL {
            let query = sqlx::query(migration);
            query.execute(&mut conn).await.unwrap();
        }

        sqlx::query("DELETE FROM comments")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM posts")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users")
            .execute(&mut conn)
            .await
            .unwrap();

        conn
    });

    (runtime, conn)
}

async fn insert_users(
    size: usize,
    conn: &mut Connection,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    if size == 0 {
        return;
    }

    let query_string = {
        #[cfg(feature = "postgres")]
        { consts::postgres::build_insert_users_query(size) }
        #[cfg(feature = "mysql")]
        { consts::mysql::build_insert_users_query(size) }
        #[cfg(feature = "sqlite")]
        { consts::mysql::build_insert_users_query(size) }
    };

    let params = consts::build_insert_users_params(size, hair_color_init);
    let mut query = sqlx::query(&query_string);
    for (name, hair_color) in &params {
        query = query.bind(name).bind(hair_color);
    }

    query.execute(conn).await.unwrap();
}

pub fn bench_trivial_query_query_as_macro(b: &mut Bencher, size: usize) {
    let (rt, mut conn) = connection();

    rt.block_on(insert_users(size, &mut conn, |_| None));

    b.iter(|| {
        rt.block_on(async {
            sqlx::query_as!(User, "SELECT id, name, hair_color FROM users")
                .fetch_all(&mut conn)
                .await
                .unwrap()
        })
    })
}

pub fn bench_trivial_query_from_row(b: &mut Bencher, size: usize) {
    let (rt, mut conn) = connection();
    rt.block_on(insert_users(size, &mut conn, |_| None));

    b.iter(|| {
        rt.block_on(async {
            sqlx::query_as::<_, User>("SELECT id, name, hair_color FROM users")
                .fetch_all(&mut conn)
                .await
                .unwrap()
        })
    })
}

// The query_as macro panics of sqlite
#[cfg(not(feature = "sqlite"))]
pub fn bench_medium_complex_query_query_as_macro(b: &mut Bencher, size: usize) {
    let (rt, mut conn) = connection();

    rt.block_on(insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    }));

    b.iter(|| {
        rt.block_on(async {
            #[cfg(feature = "postgres")]
            let res = sqlx::query_as!(UserWithPost,
                "SELECT u.id as \"myuser_id!\", u.name as \"name!\", u.hair_color, p.id as \"post_id?\", p.user_id as \"user_id?\", p.title as \"title?\", p.body  as \"body?\"\
                 FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = $1", "black"
            )
            .fetch_all(&mut conn)
            .await
            .unwrap();

            #[cfg(any(feature = "sqlite", feature = "mysql"))]
            let res = sqlx::query_as!(UserWithPost,
                "SELECT u.id as \"myuser_id!\", u.name as \"name!\", u.hair_color, p.id as \"post_id?\", p.user_id as \"user_id?\", p.title as \"title?\", p.body  as \"body?\"\
                 FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = ?", "black"
            )
            .fetch_all(&mut conn)
            .await
            .unwrap();

            res.into_iter().map(|r| {
                let user = User {
                    id: r.myuser_id,
                    name: r.name,
                    hair_color: r.hair_color,
                };
                let post = if let Some(id) = r.post_id {
                    Some(Post {
                        id,
                        title: r.title.unwrap(),
                        user_id: r.user_id.unwrap(),
                        body: r.body,
                    })
                } else {
                    None
                };
                (user, post)
            }).collect::<Vec<_>>()
        })
    })
}

pub fn bench_medium_complex_query_from_row(b: &mut Bencher, size: usize) {
    let (rt, mut conn) = connection();

    rt.block_on(insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    }));
    #[cfg(feature = "postgres")]
    let bind = "$1";
    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    let bind = "?";

    let query = format!(
        "SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id , p.title, p.body \
         FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = {bind}"
    );
    let query = &query;

    b.iter(move || {
        rt.block_on(async {
            let res = sqlx::query_as::<_, UserWithPost>(query)
                .bind("black")
                .persistent(true)
                .fetch_all(&mut conn)
                .await
                .unwrap();

            res.into_iter()
                .map(|r| {
                    let user = User {
                        id: r.myuser_id,
                        name: r.name,
                        hair_color: r.hair_color,
                    };
                    let post = if let Some(id) = r.post_id {
                        Some(Post {
                            id,
                            title: r.title.unwrap(),
                            user_id: r.user_id.unwrap(),
                            body: r.body,
                        })
                    } else {
                        None
                    };
                    (user, post)
                })
                .collect::<Vec<_>>()
        })
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let (rt, mut conn) = connection();

    b.iter(|| {
        rt.block_on(insert_users(size, &mut conn, |_| {
            Some("hair_color")
        }))
    })
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    #[cfg(feature = "sqlite")]
    const USER_NUMBER: usize = 9;

    #[cfg(not(feature = "sqlite"))]
    const USER_NUMBER: usize = 100;

    use sqlx::Connection;

    let (rt, mut conn) = connection();

    rt.block_on(insert_users(USER_NUMBER, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    }));

    let user_ids = rt.block_on(async {
        sqlx::query_as!(User, "SELECT id, name, hair_color FROM users")
            .fetch_all(&mut conn)
            .await
            .unwrap()
    });

    if cfg!(any(feature = "postgres", feature = "mysql")) {
        let data = user_ids
            .iter()
            .flat_map(|User { id: user_id, .. }| {
                (0..10).map(move |i| {
                    (
                        format!("Post {} by user {}", i, user_id),
                        user_id,
                        None::<String>,
                    )
                })
            })
            .collect::<Vec<_>>();

        let mut insert_query = String::from("INSERT INTO posts(title, user_id, body) VALUES");

        for x in 0..data.len() {
            if cfg!(feature = "postgres") {
                insert_query += &format!(
                    "{} (${}, ${}, ${})",
                    if x == 0 { "" } else { "," },
                    3 * x + 1,
                    3 * x + 2,
                    3 * x + 3
                );
            } else {
                insert_query += &format!("{} (?, ?, ?)", if x == 0 { "" } else { "," },);
            }
        }

        let mut insert_query = sqlx::query(&insert_query);

        for (title, user_id, body) in data {
            insert_query = insert_query.bind(title).bind(user_id).bind(body);
        }

        rt.block_on(async { insert_query.execute(&mut conn).await.unwrap() });
    } else if cfg!(feature = "sqlite") {
        rt.block_on(async {
            let mut conn = Connection::begin(&mut conn).await.unwrap();
            let insert_query = "INSERT INTO posts (title, user_id, body) VALUES (?, ?, ?)";

            for user in user_ids {
                for i in 0..10 {
                    let _: usize = i;
                    let insert_query = sqlx::query(insert_query)
                        .bind(format!("Post {} by user {}", i, user.id))
                        .bind(user.id)
                        .bind(None::<String>);

                    insert_query.execute(&mut *conn).await.unwrap();
                }
            }

            conn.commit().await.unwrap();
        });
    }

    let all_posts = rt.block_on(async {
        sqlx::query_as!(Post, "SELECT id, title, user_id, body FROM posts")
            .fetch_all(&mut conn)
            .await
            .unwrap()
    });

    if cfg!(any(feature = "postgres", feature = "mysql")) {
        let data = all_posts
            .iter()
            .flat_map(|Post { id: post_id, .. }| {
                (0..10).map(move |i| (format!("Comment {} on post {}", i, post_id), post_id))
            })
            .collect::<Vec<_>>();

        let mut insert_query = String::from("INSERT INTO comments(text, post_id) VALUES");

        for x in 0..data.len() {
            if cfg!(feature = "postgres") {
                insert_query += &format!(
                    "{} (${}, ${})",
                    if x == 0 { "" } else { "," },
                    2 * x + 1,
                    2 * x + 2,
                );
            } else {
                insert_query += &format!("{} (?, ?)", if x == 0 { "" } else { "," },);
            }
        }

        let mut insert_query = sqlx::query(&insert_query);

        for (title, post_id) in data {
            insert_query = insert_query.bind(title).bind(post_id);
        }

        rt.block_on(async {
            insert_query.execute(&mut conn).await.unwrap();
        });
    } else if cfg!(feature = "sqlite") {
        rt.block_on(async {
            let mut conn = Connection::begin(&mut conn).await.unwrap();
            let insert_query = "INSERT INTO comments (text, post_id) VALUES (?, ?)";

            for post in all_posts {
                for i in 0..10 {
                    let _: usize = i;
                    let insert_query = sqlx::query(insert_query)
                        .bind(format!("Comment {} on post {}", i, post.id))
                        .bind(post.id);

                    insert_query.execute(&mut *conn).await.unwrap();
                }
            }

            conn.commit().await.unwrap();
        });
    }

    b.iter(|| {
        rt.block_on(async {
            let users = sqlx::query_as!(User, "SELECT id, name, hair_color FROM users")
                .fetch_all(&mut conn)
                .await
                .unwrap();

            let mut posts_query =
                String::from("SELECT id, title, user_id, body FROM posts WHERE user_id IN(");

            for i in 0..users.len() {
                if cfg!(feature = "postgres") {
                    posts_query += &format!("{}${}", if i == 0 { "" } else { "," }, i + 1);
                } else if cfg!(not(feature = "postgres")) {
                    posts_query += &format!("{}?", if i == 0 { "" } else { "," });
                }
            }

            posts_query += ")";

            let mut posts_query = sqlx::query(&posts_query);

            for user in &users {
                posts_query = posts_query.bind(user.id);
            }

            let posts = posts_query
                .fetch_all(&mut conn)
                .await
                .unwrap()
                .into_iter()
                .map(|row| Post::from_row(&row).unwrap())
                .collect::<Vec<_>>();

            let mut comments_query =
                String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");

            for i in 0..posts.len() {
                if cfg!(feature = "postgres") {
                    comments_query += &format!("{}${}", if i == 0 { "" } else { "," }, i + 1);
                } else if cfg!(not(feature = "postgres")) {
                    comments_query += &format!("{}?", if i == 0 { "" } else { "," });
                }
            }

            comments_query += ")";

            let mut comments_query = sqlx::query(&comments_query);

            for post in &posts {
                comments_query = comments_query.bind(post.id);
            }

            let comments = comments_query
                .fetch_all(&mut conn)
                .await
                .unwrap()
                .into_iter()
                .map(|row| Comment::from_row(&row).unwrap())
                .collect::<Vec<_>>();

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
