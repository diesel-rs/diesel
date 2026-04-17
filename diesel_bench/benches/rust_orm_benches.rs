use super::consts;
use criterion::Bencher;
use rustorm::EntityManager;
use rustorm::{FromDao, Pool, ToColumnNames, ToDao, ToTableName};
use std::collections::HashMap;

mod for_insert {
    use super::*;

    #[derive(Debug, ToDao, ToColumnNames, ToTableName)]
    pub struct Users {
        pub name: String,
        pub hair_color: Option<String>,
    }

    #[derive(Debug, ToDao, ToColumnNames, ToTableName)]
    pub struct Posts {
        pub user_id: i32,
        pub title: String,
        pub body: Option<String>,
    }

    #[derive(Debug, ToDao, ToColumnNames, ToTableName)]
    pub struct Comments {
        pub text: String,
        pub post_id: i32,
    }
}

mod for_load {
    use super::*;
    use rustorm::dao::FromDao;
    use rustorm::Dao;

    #[derive(Debug, FromDao, ToColumnNames, ToTableName)]
    pub struct Users {
        pub id: i32,
        name: String,
        hair_color: Option<String>,
    }

    #[derive(Debug, FromDao, ToColumnNames, ToTableName)]
    pub struct Posts {
        pub id: i32,
        pub user_id: i32,
        title: String,
        body: Option<String>,
    }

    #[derive(Debug, FromDao, ToColumnNames, ToTableName)]
    pub struct Comments {
        id: i32,
        text: String,
        pub post_id: i32,
    }

    pub struct UserWithPost(Users, Option<Posts>);

    impl FromDao for UserWithPost {
        fn from_dao(dao: &Dao) -> Self {
            let user = Users {
                id: dao.get("myuser_id").unwrap(),
                name: dao.get("name").unwrap(),
                hair_color: dao.get("hair_color").unwrap(),
            };
            let post = if let Some(id) = dao.get("post_id").unwrap() {
                Some(Posts {
                    id,
                    user_id: dao.get("user_id").unwrap(),
                    title: dao.get("title").unwrap(),
                    body: dao.get("body").unwrap(),
                })
            } else {
                None
            };
            UserWithPost(user, post)
        }
    }
}

fn connect() -> EntityManager {
    let mut pool = Pool::new();
    dotenvy::dotenv().ok();

    let db_url = if cfg!(feature = "sqlite") {
        let url = dotenvy::var("SQLITE_DATABASE_URL")
            .or_else(|_| dotenvy::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");

        url.replace("sqlite:", "sqlite://")
    } else if cfg!(feature = "postgres") {
        dotenvy::var("POSTGRES_DATABASE_URL")
            .or_else(|_| dotenvy::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests")
    } else if cfg!(feature = "mysql") {
        dotenvy::var("MYSQL_DATABASE_URL")
            .or_else(|_| dotenvy::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests")
    } else {
        unimplemented!()
    };

    let mut entity_manager = pool.em(&db_url).unwrap();

    if cfg!(feature = "sqlite") {
        for migration in super::SQLITE_MIGRATION_SQL {
            entity_manager
                .db()
                .execute_sql_with_return(migration, &[])
                .unwrap();
        }

        entity_manager
            .db()
            .execute_sql_with_return("DELETE FROM comments", &[])
            .unwrap();
        entity_manager
            .db()
            .execute_sql_with_return("DELETE FROM posts", &[])
            .unwrap();

        entity_manager
            .db()
            .execute_sql_with_return("DELETE FROM users", &[])
            .unwrap();
    } else if cfg!(feature = "mysql") {
        #[cfg(feature = "mysql")]
        for query in consts::mysql::CLEANUP_QUERIES {
            entity_manager
                .db()
                .execute_sql_with_return(*query, &[])
                .unwrap();
        }
    } else if cfg!(feature = "postgres") {
        #[cfg(feature = "postgres")]
        for query in consts::postgres::CLEANUP_QUERIES {
            entity_manager
                .db()
                .execute_sql_with_return(*query, &[])
                .unwrap();
        }
    }

    entity_manager
}

fn insert_users(
    size: usize,
    em: &mut EntityManager,
    hair_color_init: impl Fn(usize) -> Option<String>,
) {
    let data = (0..size)
        .map(|i| for_insert::Users {
            name: format!("User {}", i),
            hair_color: hair_color_init(i),
        })
        .collect::<Vec<_>>();

    if cfg!(not(feature = "mysql")) {
        let data = data.iter().collect::<Vec<_>>();

        em.insert::<_, for_load::Users>(&data).unwrap();
    } else {
        use rustorm::Value;

        let mut query = String::from("INSERT INTO users (name, hair_color) VALUES");

        let mut params = Vec::with_capacity(data.len() * 2);

        for (i, user) in data.into_iter().enumerate() {
            params.push(Value::from(user.name));
            params.push(Value::from(user.hair_color));

            if i == 0 {
                query += "(?, ?)";
            } else {
                query += ", (?, ?)";
            }
        }

        let values = params.iter().collect::<Vec<_>>();

        em.db().execute_sql_with_return(&query, &values).unwrap();
    }
}

pub fn bench_trivial_query(b: &mut Bencher, size: usize) {
    let mut em = connect();
    insert_users(size, &mut em, |_| None);

    b.iter(|| em.get_all::<for_load::Users>().unwrap());
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let mut em = connect();
    insert_users(size, &mut em, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    #[cfg(feature = "postgres")]
    let bind = "$";
    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    let bind = "?";

    let query = format!(
        "SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, \
         p.user_id, p.title, p.body FROM users as u \
         LEFT JOIN posts as p ON u.id = p.user_id WHERE u.name = {bind}"
    );

    b.iter(|| {
        em.execute_sql_with_return::<for_load::UserWithPost>(&query, &[&"black"])
            .unwrap()
    });
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let mut em = connect();

    b.iter(|| insert_users(size, &mut em, |_| Some(String::from("hair_color"))))
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    #[cfg(feature = "sqlite")]
    const USER_NUMBER: usize = 9;

    #[cfg(not(feature = "sqlite"))]
    const USER_NUMBER: usize = 100;

    // SETUP A TON OF DATA
    let mut em = connect();
    insert_users(USER_NUMBER, &mut em, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let all_users = em.get_all::<for_load::Users>().unwrap();

    let data: Vec<_> = all_users
        .iter()
        .flat_map(|user| {
            let user_id = user.id;
            (0..10).map(move |i| {
                let title = format!("Post {} by user {}", i, user_id);
                for_insert::Posts {
                    user_id,
                    title,
                    body: None,
                }
            })
        })
        .collect();

    let all_posts = if cfg!(not(feature = "mysql")) {
        let data = data.iter().collect::<Vec<_>>();

        em.insert::<_, for_load::Posts>(&data).unwrap()
    } else {
        use rustorm::Value;

        let mut query = String::from("INSERT INTO posts (user_id, title, body) VALUES");

        let mut params = Vec::with_capacity(data.len() * 3);

        for (i, post) in data.into_iter().enumerate() {
            params.push(Value::from(post.user_id));
            params.push(Value::from(post.title));
            params.push(Value::from(post.body));

            if i == 0 {
                query += "(?, ?, ?)";
            } else {
                query += ", (?, ?, ?)";
            }
        }

        let values = params.iter().collect::<Vec<_>>();

        em.db().execute_sql_with_return(&query, &values).unwrap();

        em.get_all::<for_load::Posts>().unwrap()
    };

    let data: Vec<_> = all_posts
        .iter()
        .flat_map(|post| {
            let post_id = post.id;
            (0..10).map(move |i| {
                let title = format!("Comment {} on post {}", i, post_id);
                for_insert::Comments {
                    text: title,
                    post_id,
                }
            })
        })
        .collect();
    if cfg!(not(feature = "mysql")) {
        let data = data.iter().collect::<Vec<_>>();

        let _ = em.insert::<_, for_load::Comments>(&data).unwrap();
    } else {
        use rustorm::Value;

        let mut query = String::from("INSERT INTO comments (text, post_id) VALUES");

        let mut params = Vec::with_capacity(data.len() * 2);

        for (i, comment) in data.into_iter().enumerate() {
            params.push(Value::from(comment.text));
            params.push(Value::from(comment.post_id));

            if i == 0 {
                query += "(?, ?)";
            } else {
                query += ", (?, ?)";
            }
        }

        let values = params.iter().collect::<Vec<_>>();

        em.db().execute_sql_with_return(&query, &values).unwrap();
    }

    b.iter(|| {
        let users = em.get_all::<for_load::Users>().unwrap();

        let mut params = Vec::with_capacity(users.len());

        let mut posts_query =
            String::from("SELECT id, title, user_id, body FROM posts WHERE user_id IN(");

        for (i, user) in users.iter().enumerate() {
            if cfg!(feature = "postgres") {
                posts_query += &format!("{}${}", if i == 0 { "" } else { "," }, i + 1);
            } else if cfg!(not(feature = "postgres")) {
                posts_query += &format!("{}?", if i == 0 { "" } else { "," });
            }

            params.push(&user.id as _);
        }

        posts_query += ")";

        let posts = em
            .execute_sql_with_return::<for_load::Posts>(&posts_query, &params)
            .unwrap();

        let mut comments_query =
            String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");

        let mut params = Vec::with_capacity(posts.len());

        for (i, post) in posts.iter().enumerate() {
            if cfg!(feature = "postgres") {
                comments_query += &format!("{}${}", if i == 0 { "" } else { "," }, i + 1);
            } else if cfg!(not(feature = "postgres")) {
                comments_query += &format!("{}?", if i == 0 { "" } else { "," });
            }

            params.push(&post.id as _);
        }

        comments_query += ")";

        let comments = em
            .execute_sql_with_return::<for_load::Comments>(&comments_query, &params)
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
            .collect::<Vec<(
                for_load::Users,
                Vec<(for_load::Posts, Vec<for_load::Comments>)>,
            )>>()
    })
}
