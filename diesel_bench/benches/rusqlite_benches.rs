use criterion::Bencher;
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::Row;
use rusqlite::ToSql;
use std::collections::HashMap;

pub struct User {
    pub id: i64,
    pub name: String,
    pub hair_color: Option<String>,
}

impl User {
    fn from_row_by_id(row: &Row) -> User {
        User {
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap(),
            hair_color: row.get(2).unwrap(),
        }
    }

    fn from_row_by_name(row: &Row) -> User {
        User {
            id: row.get("id").unwrap(),
            name: row.get("name").unwrap(),
            hair_color: row.get("hair_color").unwrap(),
        }
    }
}

pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub body: Option<String>,
}

impl Post {
    fn from_row_by_id(row: &Row) -> Post {
        Post {
            id: row.get(0).unwrap(),
            title: row.get(1).unwrap(),
            user_id: row.get(2).unwrap(),
            body: row.get(3).unwrap(),
        }
    }
}

pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub text: String,
}

impl Comment {
    fn from_row_by_id(row: &Row) -> Comment {
        Comment {
            id: row.get(0).unwrap(),
            post_id: row.get(1).unwrap(),
            text: row.get(2).unwrap(),
        }
    }
}

fn connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    for migration in super::SQLITE_MIGRATION_SQL {
        conn.execute(migration, []).unwrap();
    }

    conn.execute("DELETE FROM comments", []).unwrap();
    conn.execute("DELETE FROM posts", []).unwrap();
    conn.execute("DELETE FROM users", []).unwrap();

    conn
}

fn insert_users(
    size: usize,
    conn: &mut Connection,
    hair_color_init: impl Fn(usize) -> Option<String>,
) {
    if size == 0 {
        return;
    }

    let conn = conn.transaction().unwrap();

    {
        let mut query = conn
            .prepare("INSERT INTO users (name, hair_color) VALUES (?, ?)")
            .unwrap();

        for x in 0..size {
            query
                .execute(params!(format!("User {}", x), hair_color_init(x)))
                .unwrap();
        }
    }

    conn.commit().unwrap();
}

pub fn bench_trivial_query_by_id(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let mut query = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        query
            .query_map([], |row| Ok(User::from_row_by_id(row)))
            .unwrap()
            .collect::<Vec<_>>()
    });
}

pub fn bench_trivial_query_by_name(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    let mut query = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        query
            .query_map([], |row| Ok(User::from_row_by_name(row)))
            .unwrap()
            .collect::<Vec<_>>()
    });
}

pub fn bench_medium_complex_query_by_id(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let mut query = conn.prepare(
        "SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id , p.title, p.body \
         FROM users as u LEFT JOIN posts as p on u.id = p.user_id"
    ).unwrap();

    b.iter(|| {
        query
            .query_map([], |row| {
                let user = User::from_row_by_id(row);
                let post = if let Some(id) = row.get(4).unwrap() {
                    Some(Post {
                        id,
                        user_id: row.get(5).unwrap(),
                        title: row.get(6).unwrap(),
                        body: row.get(7).unwrap(),
                    })
                } else {
                    None
                };
                Ok((user, post))
            })
            .unwrap()
            .collect::<Vec<_>>()
    })
}

pub fn bench_medium_complex_query_by_name(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" }.into())
    });

    let mut query = conn.prepare(
        "SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id , p.title, p.body \
         FROM users as u LEFT JOIN posts as p on u.id = p.user_id"
    ).unwrap();

    b.iter(|| {
        query
            .query_map([], |row| {
                let user = User {
                    id: row.get("myuser_id").unwrap(),
                    name: row.get("name").unwrap(),
                    hair_color: row.get("hair_color").unwrap(),
                };
                let post = if let Some(id) = row.get("post_id").unwrap() {
                    Some(Post {
                        id,
                        user_id: row.get("user_id").unwrap(),
                        title: row.get("title").unwrap(),
                        body: row.get("body").unwrap(),
                    })
                } else {
                    None
                };
                Ok((user, post))
            })
            .unwrap()
            .collect::<Vec<_>>()
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let mut conn = connection();

    b.iter(|| insert_users(size, &mut conn, |_| Some(String::from("hair_color"))))
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    let mut conn = connection();

    insert_users(9, &mut conn, |i| {
        Some(if i % 2 == 0 {
            String::from("black")
        } else {
            String::from("brown")
        })
    });

    let user_ids = {
        let mut user_query = conn.prepare("SELECT id FROM users").unwrap();

        user_query
            .query_map([], |row| Ok(row.get("id").unwrap()))
            .unwrap()
            .collect::<Result<Vec<i32>, _>>()
            .unwrap()
    };

    {
        let conn = conn.transaction().unwrap();
        {
            let mut insert_posts = conn
                .prepare("INSERT INTO posts(title, user_id, body) VALUES (?, ?, ?)")
                .unwrap();

            for user_id in user_ids {
                for i in 0..10 {
                    insert_posts
                        .execute(params!(
                            format!("Post {} by user {}", i, user_id),
                            user_id,
                            None::<String>
                        ))
                        .unwrap();
                }
            }
        }

        conn.commit().unwrap();
    }

    let all_posts = {
        let mut post_query = conn.prepare("SELECT id FROM posts").unwrap();

        post_query
            .query_map([], |row| Ok(row.get("id").unwrap()))
            .unwrap()
            .collect::<Result<Vec<i32>, _>>()
            .unwrap()
    };

    {
        let conn = conn.transaction().unwrap();
        {
            let mut insert_comments = conn
                .prepare("INSERT INTO comments(text, post_id) VALUES (?, ?)")
                .unwrap();

            for post_id in all_posts {
                for i in 0..10 {
                    insert_comments
                        .execute(params!(
                            format!("Comment {} for post {}", i, post_id),
                            post_id
                        ))
                        .unwrap();
                }
            }
        }

        conn.commit().unwrap();
    }

    let mut user_query = conn
        .prepare("SELECT id, name, hair_color FROM users")
        .unwrap();

    b.iter(|| {
        let users = user_query
            .query_map([], |row| Ok(User::from_row_by_id(row)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let mut posts_query =
            String::from("SELECT id, title, user_id, body FROM posts WHERE user_id IN(");

        let user_ids = users
            .iter()
            .enumerate()
            .map(|(i, &User { ref id, .. })| {
                posts_query += &format!("{}?", if i == 0 { "" } else { "," });
                id as &dyn ToSql
            })
            .collect::<Vec<_>>();

        posts_query += ")";

        let mut posts_query = conn.prepare(&posts_query).unwrap();

        let posts = posts_query
            .query_map(&user_ids as &[_], |row| Ok(Post::from_row_by_id(row)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let mut comments_query =
            String::from("SELECT id, post_id, text FROM comments WHERE post_id IN(");

        let post_ids = posts
            .iter()
            .enumerate()
            .map(|(i, &Post { ref id, .. })| {
                comments_query += &format!("{}?", if i == 0 { "" } else { "," });
                id as &dyn ToSql
            })
            .collect::<Vec<_>>();

        comments_query += ")";

        let mut comments_query = conn.prepare(&comments_query).unwrap();

        let comments = comments_query
            .query_map(&post_ids as &[_], |row| Ok(Comment::from_row_by_id(row)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
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
