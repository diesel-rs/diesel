use super::consts;
use super::Bencher;
use sea_orm::entity::*;
use sea_orm::query::*;
use sea_orm::DatabaseConnection;
use tokio::runtime::Runtime;

mod comments;
mod posts;
mod users;

use self::comments::Entity as Comment;
use self::posts::Entity as Post;
use self::users::Entity as User;

#[cfg(feature = "postgres")]
fn connection() -> (sqlx::PgPool, DatabaseConnection, Runtime) {
    use sea_orm::SqlxPostgresConnector;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to start runtime");

    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("PG_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let (pool, db) = rt.block_on(async {
        use sqlx::Executor;
        let pool = sqlx::PgPool::connect(&connection_url).await.unwrap();
        for query in consts::postgres::CLEANUP_QUERIES {
            pool.execute(*query).await.unwrap();
        }

        let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool.clone());
        (pool, db)
    });

    (pool, db, rt)
}

#[cfg(feature = "mysql")]
fn connection() -> (sqlx::MySqlPool, DatabaseConnection, Runtime) {
    use futures::StreamExt;
    use sea_orm::SqlxMySqlConnector;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to start runtime");

    dotenvy::dotenv().ok();
    let connection_url = dotenvy::var("MYSQL_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let (pool, db) = rt.block_on(async {
        use sqlx::Executor;
        let pool = sqlx::MySqlPool::connect(&connection_url).await.unwrap();
        let cleanup = consts::mysql::CLEANUP_QUERIES.join(";");
        let mut result_stream = pool.execute_many(cleanup.as_str());
        while let Some(e) = result_stream.next().await {
            let _ = e.unwrap();
        }
        let db = SqlxMySqlConnector::from_sqlx_mysql_pool(pool.clone());
        (pool, db)
    });
    (pool, db, rt)
}

#[cfg(feature = "sqlite")]
fn connection() -> (sqlx::SqlitePool, DatabaseConnection, Runtime) {
    use sea_orm::SqlxSqliteConnector;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to start runtime");
    dotenvy::dotenv().ok();

    let (pool, db) = rt.block_on(async {
        use sqlx::Executor;
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();

        for migration in super::SQLITE_MIGRATION_SQL {
            pool.execute(*migration).await.unwrap();
        }

        pool.execute("DELETE FROM comments").await.unwrap();
        pool.execute("DELETE FROM posts").await.unwrap();
        pool.execute("DELETE FROM users").await.unwrap();

        let db = SqlxSqliteConnector::from_sqlx_sqlite_pool(pool.clone());
        (pool, db)
    });

    (pool, db, rt)
}

async fn insert_users(
    size: usize,
    conn: &DatabaseConnection,
    hair_color_init: impl Fn(usize) -> Option<String>,
) {
    let values = (0..size)
        .map(|idx| self::users::ActiveModel {
            name: Set(format!("User {}", idx)),
            hair_color: Set(hair_color_init(idx)),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    User::insert_many(values).exec(conn).await.unwrap();
}

pub fn bench_trivial_query(b: &mut Bencher, size: usize) {
    let (pool, conn, rt) = connection();

    rt.block_on(async {
        insert_users(size, &conn, |_| None).await;
    });

    b.to_async(&rt)
        .iter(|| async { User::find().all(&conn).await.unwrap() });

    rt.block_on(async {
        pool.close().await;
    })
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let (pool, conn, rt) = connection();

    let hair_color_callback = |i| {
        Some(if i % 2 == 0 {
            String::from("black")
        } else {
            String::from("brown")
        })
    };
    rt.block_on(async {
        insert_users(size, &conn, hair_color_callback).await;
    });

    b.to_async(&rt).iter(|| async {
        let r: Vec<(self::users::Model, Option<self::posts::Model>)> = User::find()
            .find_also_related(Post)
            .filter(self::users::Column::HairColor.eq("black"))
            .all(&conn)
            .await
            .unwrap();
        r
    });

    rt.block_on(async {
        pool.close().await;
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let (pool, conn, rt) = connection();

    b.to_async(&rt).iter(|| async {
        insert_users(size, &conn, |_| Some(String::from("hair_color"))).await;
    });

    rt.block_on(async {
        pool.close().await;
    })
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    #[cfg(feature = "sqlite")]
    const USER_NUMBER: usize = 9;

    #[cfg(not(feature = "sqlite"))]
    const USER_NUMBER: usize = 100;

    // SETUP A TON OF DATA
    let (pool, conn, rt) = connection();

    rt.block_on(async {
        insert_users(USER_NUMBER, &conn, |i| {
            Some(if i % 2 == 0 {
                "black".to_owned()
            } else {
                "brown".to_owned()
            })
        })
        .await;

        let all_users = User::find().all(&conn).await.unwrap();
        let data: Vec<_> = all_users
            .iter()
            .flat_map(|user| {
                let user_id = user.id;
                (0..10).map(move |i| {
                    let title = format!("Post {} by user {}", i, user_id);
                    self::posts::ActiveModel {
                        user_id: Set(user_id),
                        title: Set(title),
                        body: Set(None),
                        ..Default::default()
                    }
                })
            })
            .collect();
        Post::insert_many(data).exec(&conn).await.unwrap();
        let all_posts = Post::find().all(&conn).await.unwrap();
        let data: Vec<_> = all_posts
            .iter()
            .flat_map(|post| {
                let post_id = post.id;
                (0..10).map(move |i| {
                    let title = format!("Comment {} on post {}", i, post_id);
                    self::comments::ActiveModel {
                        text: Set(title),
                        post_id: Set(post_id),
                        ..Default::default()
                    }
                })
            })
            .collect();
        Comment::insert_many(data).exec(&conn).await.unwrap();
    });

    // ACTUAL BENCHMARK
    b.to_async(&rt).iter(|| async {
        use std::collections::HashMap;

        let res: Vec<(self::users::Model, Vec<self::posts::Model>)> = User::find()
            .find_with_related(Post)
            .all(&conn)
            .await
            .unwrap();

        let post_ids = res
            .iter()
            .flat_map(|(_, posts)| posts.iter().map(|post| post.id))
            .collect::<Vec<_>>();

        let comments = Comment::find()
            .filter(self::comments::Column::PostId.is_in(post_ids))
            .all(&conn)
            .await
            .unwrap();

        let mut lookup = HashMap::new();

        for comment in comments {
            lookup
                .entry(comment.post_id)
                .or_insert(Vec::new())
                .push(comment);
        }

        res.into_iter()
            .map(|(user, posts)| {
                let posts = posts
                    .into_iter()
                    .map(|post| {
                        let post_id = post.id;
                        (post, lookup[&post_id].clone())
                    })
                    .collect();
                (user, posts)
            })
            .collect::<Vec<(
                self::users::Model,
                Vec<(self::posts::Model, Vec<self::comments::Model>)>,
            )>>()
    });

    rt.block_on(async {
        pool.close().await;
    })
}
