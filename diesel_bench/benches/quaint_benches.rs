use criterion::Bencher;
use quaint::prelude::*;
use quaint::single::Quaint;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::runtime::Runtime;

#[derive(Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

#[derive(Deserialize)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

#[derive(Deserialize)]
pub struct Comment {
    pub id: i32,
    pub post_id: i32,
    pub text: String,
}

#[derive(Deserialize)]
pub struct UserWithPost {
    pub myuser_id: i32,
    pub name: String,
    pub hair_color: Option<String>,
    pub post_id: Option<i32>,
    pub user_id: Option<i32>,
    pub title: Option<String>,
    pub body: Option<String>,
}

fn connect(rt: &mut Runtime) -> Quaint {
    dotenv::dotenv().ok();
    let db_url = if cfg!(feature = "sqlite") {
        dotenv::var("SQLITE_DATABASE_URL")
            .or_else(|_| dotenv::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests")
    } else if cfg!(feature = "postgres") {
        dotenv::var("POSTGRES_DATABASE_URL")
            .or_else(|_| dotenv::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests")
    } else if cfg!(feature = "mysql") {
        dotenv::var("MYSQL_DATABASE_URL")
            .or_else(|_| dotenv::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests")
    } else {
        unimplemented!()
    };

    let conn = rt.block_on({
        async {
            let conn = Quaint::new(&db_url).await.unwrap();

            if cfg!(feature = "sqlite") {
                for migration in super::SQLITE_MIGRATION_SQL {
                    conn.execute_raw(migration, &[]).await.unwrap();
                }
                conn.execute_raw("DELETE FROM comments;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("DELETE FROM posts;", &[]).await.unwrap();
                conn.execute_raw("DELETE FROM users;", &[]).await.unwrap();
            } else if cfg!(feature = "postgres") {
                conn.execute_raw("TRUNCATE TABLE comments CASCADE;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("TRUNCATE TABLE posts CASCADE;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("TRUNCATE TABLE users CASCADE;", &[])
                    .await
                    .unwrap();
            } else if cfg!(feature = "mysql") {
                conn.execute_raw("SET FOREIGN_KEY_CHECKS = 0;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("TRUNCATE TABLE comments;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("TRUNCATE TABLE posts;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("TRUNCATE TABLE users;", &[])
                    .await
                    .unwrap();
                conn.execute_raw("SET FOREIGN_KEY_CHECKS = 1;", &[])
                    .await
                    .unwrap();
            }
            conn
        }
    });

    conn
}

fn insert_users(
    size: usize,
    conn: &mut Quaint,
    hair_color_init: impl Fn(usize) -> Option<String>,
    rt: &mut Runtime,
) {
    if size == 0 {
        return;
    }

    if cfg!(feature = "sqlite") {
        rt.block_on(async {
            let transaction = conn.start_transaction().await.unwrap();

            for i in 0..size {
                let insert = Insert::single_into("users")
                    .value("name", format!("User {}", i))
                    .value("hair_color", hair_color_init(i));
                conn.insert(insert.build()).await.unwrap();
            }

            transaction.commit().await.unwrap();
        });
    } else {
        let mut insert = Insert::multi_into("users", vec!["name", "hair_color"]);

        for x in 0..size {
            insert = insert.values((format!("User {}", x), hair_color_init(x)));
        }

        rt.block_on(async { conn.insert(insert.build()).await.unwrap() });
    }
}

pub fn bench_trivial_query(b: &mut Bencher, size: usize) {
    let mut rt = Runtime::new().unwrap();
    let mut conn = connect(&mut rt);
    insert_users(size, &mut conn, |_| None, &mut rt);

    b.iter(|| {
        rt.block_on(async {
            let select = Select::from_table("users");
            let result_set = conn.select(select).await.unwrap();
            quaint::serde::from_rows::<User>(result_set).unwrap()
        })
    })
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let mut rt = Runtime::new().unwrap();
    let mut conn = connect(&mut rt);
    insert_users(
        size,
        &mut conn,
        |i| Some(if i % 2 == 0 { "black" } else { "brown" }.into()),
        &mut rt,
    );

    b.iter(|| {
        rt.block_on(async {
            let select = Select::from_table("users".alias("u"))
                .left_join(
                    "posts"
                        .alias("p")
                        .on(("u", "id").equals(Column::from(("p", "user_id")))),
                )
                .columns(vec![
                    Column::from(("u", "id")).alias("myuser_id"),
                    ("u", "name").into(),
                    ("u", "hair_color").into(),
                    Column::from(("p", "id")).alias("post_id"),
                    ("p", "user_id").into(),
                    ("p", "title").into(),
                    ("p", "body").into(),
                ]);
            let result_set = conn.select(select).await.unwrap();
            quaint::serde::from_rows::<UserWithPost>(result_set)
                .unwrap()
                .into_iter()
                .map(|i| {
                    let user = User {
                        id: i.myuser_id,
                        name: i.name,
                        hair_color: i.hair_color,
                    };
                    let post = if let Some(id) = i.post_id {
                        Some(Post {
                            id,
                            user_id: i.user_id.unwrap(),
                            title: i.title.unwrap(),
                            body: i.body,
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
    let mut rt = Runtime::new().unwrap();
    let mut conn = connect(&mut rt);

    b.iter(|| {
        insert_users(
            size,
            &mut conn,
            |_| Some(String::from("hair_color")),
            &mut rt,
        )
    })
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    let mut rt = Runtime::new().unwrap();
    let mut conn = connect(&mut rt);

    #[cfg(feature = "sqlite")]
    const USER_NUMBER: usize = 9;

    #[cfg(not(feature = "sqlite"))]
    const USER_NUMBER: usize = 100;

    insert_users(
        USER_NUMBER,
        &mut conn,
        |i| Some(if i % 2 == 0 { "black" } else { "brown" }.into()),
        &mut rt,
    );

    let all_users = rt.block_on(async {
        let select = Select::from_table("users");
        let result_set = conn.select(select).await.unwrap();
        quaint::serde::from_rows::<User>(result_set).unwrap()
    });

    if cfg!(not(feature = "sqlite")) {
        let mut insert_posts = Insert::multi_into("posts", vec!["title", "user_id", "body"]);

        for user in all_users {
            for i in 0..10 {
                insert_posts = insert_posts.values((
                    format!("Post {} by user {}", i, user.id),
                    user.id,
                    None::<String>,
                ));
            }
        }

        rt.block_on(async { conn.insert(insert_posts.build()).await.unwrap() });
    } else {
        rt.block_on(async {
            let transaction = conn.start_transaction().await.unwrap();

            for user in all_users {
                for i in 0..10 {
                    let i: usize = i;
                    let insert = Insert::single_into("posts")
                        .value("title", format!("Post {} by user {}", i, user.id))
                        .value("user_id", user.id)
                        .value("body", None::<String>);

                    transaction.insert(insert.build()).await.unwrap();
                }
            }

            transaction.commit().await.unwrap();
        });
    }

    let all_posts = rt.block_on(async {
        let select = Select::from_table("posts");
        let result_set = conn.select(select).await.unwrap();
        quaint::serde::from_rows::<Post>(result_set).unwrap()
    });

    if cfg!(not(feature = "sqlite")) {
        let mut insert_comments = Insert::multi_into("comments", vec!["text", "post_id"]);

        for post in all_posts {
            for i in 0..10 {
                insert_comments =
                    insert_comments.values((format!("Comment {} on post {}", i, post.id), post.id));
            }
        }

        rt.block_on(async { conn.insert(insert_comments.build()).await.unwrap() });
    } else {
        rt.block_on(async {
            let transaction = conn.start_transaction().await.unwrap();

            for post in all_posts {
                for i in 0..10 {
                    let i: usize = i;
                    let insert = Insert::single_into("comments")
                        .value("text", format!("Comment {} on post {}", i, post.id))
                        .value("post_id", post.id);
                    transaction.insert(insert.build()).await.unwrap();
                }
            }

            transaction.commit().await.unwrap();
        });
    }

    b.iter(|| {
        rt.block_on(async {
            let user_query = Select::from_table("users");
            let users =
                quaint::serde::from_rows::<User>(conn.select(user_query).await.unwrap()).unwrap();
            let user_ids = users.iter().map(|user| user.id).collect::<Vec<_>>();

            let posts_query =
                Select::from_table("posts").and_where("user_id".in_selection(user_ids));
            let posts =
                quaint::serde::from_rows::<Post>(conn.select(posts_query).await.unwrap()).unwrap();
            let post_ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();

            let comments_query =
                Select::from_table("comments").and_where("post_id".in_selection(post_ids));
            let comments =
                quaint::serde::from_rows::<Comment>(conn.select(comments_query).await.unwrap())
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
    });
}
