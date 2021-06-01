use criterion::Bencher;
use diesel::*;

#[cfg(feature = "postgres")]
type TestConnection = PgConnection;

#[cfg(feature = "mysql")]
type TestConnection = MysqlConnection;

#[cfg(feature = "sqlite")]
type TestConnection = SqliteConnection;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
    }
}

table! {
    comments {
        id -> Integer,
        post_id -> Integer,
        text -> Text,
    }
}

joinable!(comments -> posts (post_id));
joinable!(posts -> users (user_id));
allow_tables_to_appear_in_same_query!(users, posts, comments);

#[derive(
    PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Insertable, AsChangeset, QueryableByName,
)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Queryable, Clone, Insertable, AsChangeset)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
    pub hair_color: Option<String>,
}

impl NewUser {
    pub fn new(name: &str, hair_color: Option<&str>) -> Self {
        NewUser {
            name: name.to_string(),
            hair_color: hair_color.map(|s| s.to_string()),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations, QueryableByName)]
#[belongs_to(User)]
#[table_name = "posts"]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

#[derive(Insertable)]
#[table_name = "posts"]
pub struct NewPost {
    user_id: i32,
    title: String,
    body: Option<String>,
}

impl NewPost {
    pub fn new(user_id: i32, title: &str, body: Option<&str>) -> Self {
        NewPost {
            user_id,
            title: title.into(),
            body: body.map(|b| b.into()),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(Post)]
pub struct Comment {
    id: i32,
    post_id: i32,
    text: String,
}

#[derive(Debug, Clone, Copy, Insertable)]
#[table_name = "comments"]
pub struct NewComment<'a>(
    #[column_name = "post_id"] pub i32,
    #[column_name = "text"] pub &'a str,
);

#[cfg(feature = "mysql")]
fn connection() -> TestConnection {
    dotenv::dotenv().ok();
    let connection_url = dotenv::var("MYSQL_DATABASE_URL")
        .or_else(|_| dotenv::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut conn = MysqlConnection::establish(&connection_url).unwrap();
    conn.execute("SET FOREIGN_KEY_CHECKS = 0;").unwrap();
    conn.execute("TRUNCATE TABLE comments").unwrap();
    conn.execute("TRUNCATE TABLE posts").unwrap();
    conn.execute("TRUNCATE TABLE users").unwrap();
    conn.execute("SET FOREIGN_KEY_CHECKS = 1;").unwrap();
    conn
}

#[cfg(feature = "postgres")]
fn connection() -> TestConnection {
    dotenv::dotenv().ok();
    let connection_url = dotenv::var("PG_DATABASE_URL")
        .or_else(|_| dotenv::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut conn = PgConnection::establish(&connection_url).unwrap();
    conn.execute("TRUNCATE TABLE comments CASCADE").unwrap();
    conn.execute("TRUNCATE TABLE posts CASCADE").unwrap();
    conn.execute("TRUNCATE TABLE users CASCADE").unwrap();
    conn
}

#[cfg(feature = "sqlite")]
fn connection() -> TestConnection {
    dotenv::dotenv().ok();
    let mut conn = diesel::SqliteConnection::establish(":memory:").unwrap();
    for migration in super::SQLITE_MIGRATION_SQL {
        conn.execute(migration).unwrap();
    }
    conn.execute("DELETE FROM comments").unwrap();
    conn.execute("DELETE FROM posts").unwrap();
    conn.execute("DELETE FROM users").unwrap();
    conn
}

fn insert_users(
    size: usize,
    conn: &mut TestConnection,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) {
    let data: Vec<_> = (0..size)
        .map(|i| NewUser::new(&format!("User {}", i), hair_color_init(i)))
        .collect();
    insert_into(users::table)
        .values(&data)
        .execute(conn)
        .unwrap();
}

pub fn bench_trivial_query(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    b.iter(|| users::table.load::<User>(&mut conn).unwrap())
}

pub fn bench_trivial_query_boxed(b: &mut Bencher, size: usize) {
    let mut conn = connection();

    insert_users(size, &mut conn, |_| None);
    b.iter(|| users::table.into_boxed().load::<User>(&mut conn).unwrap())
}

pub fn bench_trivial_query_raw(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |_| None);

    b.iter(|| {
        diesel::sql_query("SELECT id, name, hair_color FROM users")
            .load::<User>(&mut conn)
            .unwrap()
    })
}

pub fn bench_medium_complex_query(b: &mut Bencher, size: usize) {
    let mut conn = connection();

    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    });

    b.iter(|| {
        use self::users::dsl::*;
        let target = users
            .left_outer_join(posts::table)
            .filter(hair_color.eq("black"));
        target.load::<(User, Option<Post>)>(&mut conn).unwrap()
    })
}

pub fn bench_medium_complex_query_boxed(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    });

    b.iter(|| {
        use self::users::dsl::*;
        let target = users
            .left_outer_join(posts::table)
            .filter(hair_color.eq("black"))
            .into_boxed();
        target.load::<(User, Option<Post>)>(&mut conn).unwrap()
    })
}

pub fn bench_medium_complex_query_queryable_by_name(b: &mut Bencher, size: usize) {
    let mut conn = connection();
    insert_users(size, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    });

    b.iter(|| {
        diesel::sql_query(
            "SELECT u.id, u.name, u.hair_color, p.id, p.user_id, p.title, p.body \
             FROM users as u LEFT JOIN posts as p on u.id = p.user_id",
        )
        .load::<(User, Option<Post>)>(&mut conn)
        .unwrap()
    })
}

pub fn bench_insert(b: &mut Bencher, size: usize) {
    let mut conn = connection();

    b.iter(|| insert_users(size, &mut conn, |_| Some("hair_color")))
}

pub fn loading_associations_sequentially(b: &mut Bencher) {
    #[cfg(feature = "sqlite")]
    const USER_NUMBER: usize = 9;

    #[cfg(not(feature = "sqlite"))]
    const USER_NUMBER: usize = 100;

    // SETUP A TON OF DATA
    let mut conn = connection();
    insert_users(USER_NUMBER, &mut conn, |i| {
        Some(if i % 2 == 0 { "black" } else { "brown" })
    });

    let all_users = users::table.load::<User>(&mut conn).unwrap();
    let data: Vec<_> = all_users
        .iter()
        .flat_map(|user| {
            let user_id = user.id;
            (0..10).map(move |i| {
                let title = format!("Post {} by user {}", i, user_id);
                NewPost::new(user_id, &title, None)
            })
        })
        .collect();
    insert_into(posts::table)
        .values(&data)
        .execute(&mut conn)
        .unwrap();
    let all_posts = posts::table.load::<Post>(&mut conn).unwrap();
    let data: Vec<_> = all_posts
        .iter()
        .flat_map(|post| {
            let post_id = post.id;
            (0..10).map(move |i| {
                let title = format!("Comment {} on post {}", i, post_id);
                (title, post_id)
            })
        })
        .collect();
    let comment_data: Vec<_> = data
        .iter()
        .map(|&(ref title, post_id)| NewComment(post_id, &title))
        .collect();
    insert_into(comments::table)
        .values(&comment_data)
        .execute(&mut conn)
        .unwrap();

    // ACTUAL BENCHMARK
    b.iter(|| {
        let users = users::table.load::<User>(&mut conn).unwrap();
        let posts = Post::belonging_to(&users).load::<Post>(&mut conn).unwrap();
        let comments = Comment::belonging_to(&posts)
            .load::<Comment>(&mut conn)
            .unwrap()
            .grouped_by(&posts);
        let posts_and_comments = posts.into_iter().zip(comments).grouped_by(&users);
        users
            .into_iter()
            .zip(posts_and_comments)
            .collect::<Vec<(User, Vec<(Post, Vec<Comment>)>)>>()
    })
}
