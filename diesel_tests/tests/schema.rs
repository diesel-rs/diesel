use diesel::*;
use dotenv::dotenv;
use std::env;

#[cfg(all(feature="postgres", feature="backend_specific_database_url"))]
infer_schema!("dotenv:PG_DATABASE_URL");
#[cfg(all(feature="sqlite", feature="backend_specific_database_url"))]
infer_schema!("dotenv:SQLITE_DATABASE_URL");
#[cfg(all(feature="mysql", feature="backend_specific_database_url"))]
infer_schema!("dotenv:MYSQL_DATABASE_URL");
#[cfg(not(feature="backend_specific_database_url"))]
infer_schema!("dotenv:DATABASE_URL");

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Insertable, AsChangeset, Associations)]
#[has_many(posts)]
#[has_many(likes)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

impl User {
    pub fn new(id: i32, name: &str) -> Self {
        User { id: id, name: name.to_string(), hair_color: None }
    }

    pub fn with_hair_color(id: i32, name: &str, hair_color: &str) -> Self {
        User {
            id: id,
            name: name.to_string(),
            hair_color: Some(hair_color.to_string()),
        }
    }

    pub fn new_post(&self, title: &str, body: Option<&str>) -> NewPost {
        NewPost::new(self.id, title, body)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(Post)]
#[has_many(likes)]
pub struct Comment {
    id: i32,
    post_id: i32,
    text: String,
}

impl Comment {
    pub fn new(id: i32, post_id: i32, text: &str) -> Self {
        Comment {
            id: id,
            post_id: post_id,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Insertable)]
#[table_name="followings"]
pub struct Following {
    pub user_id: i32,
    pub post_id: i32,
    pub email_notifications: bool,
}

impl ::diesel::associations::HasTable for Following {
    type Table = followings::table;

    fn table() -> Self::Table {
        followings::table
    }
}

// Test to ensure a proper implementation of `Identifiable` can be written for composite types.
// This will eventually be replaced with a derive.
impl<'a> ::diesel::associations::Identifiable for &'a Following {
    type Id = (&'a i32, &'a i32);

    fn id(self) -> Self::Id {
        (&self.user_id, &self.post_id)
    }
}

#[cfg_attr(feature="postgres", path="postgres_specific_schema.rs")]
#[cfg_attr(not(feature="postgres"), path="backend_specific_schema.rs")]
mod backend_specifics;

pub use self::backend_specifics::*;

numeric_expr!(users::id);

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

#[derive(Insertable)]
#[table_name="posts"]
pub struct NewPost {
    user_id: i32,
    title: String,
    body: Option<String>,
}

impl NewPost {
    pub fn new(user_id: i32, title: &str, body: Option<&str>) -> Self {
        NewPost {
            user_id: user_id,
            title: title.into(),
            body: body.map(|b| b.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Insertable)]
#[table_name="comments"]
pub struct NewComment<'a>(
    #[column_name(post_id)]
    pub i32,
    #[column_name(text)]
    pub &'a str,
);

#[derive(PartialEq, Eq, Debug, Clone, Insertable, Associations)]
#[table_name="fk_tests"]
pub struct FkTest {
    id: i32,
    fk_id: i32,
}

impl FkTest {
    pub fn new(id: i32, fk_id: i32) -> Self {
        FkTest{ id: id, fk_id: fk_id }
    }
}

numeric_expr!(nullable_table::value);

#[derive(Queryable, Insertable)]
#[table_name="nullable_table"]
pub struct NullableColumn {
    id: i32,
    value: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Queryable, Insertable, Identifiable, Associations)]
#[table_name="likes"]
#[primary_key(user_id, comment_id)]
#[belongs_to(User)]
#[belongs_to(Comment)]
pub struct Like {
    pub user_id: i32,
    pub comment_id: i32,
}

#[cfg(feature = "postgres")]
pub type TestConnection = ::diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
pub type TestConnection = ::diesel::sqlite::SqliteConnection;
#[cfg(feature = "mysql")]
pub type TestConnection = ::diesel::mysql::MysqlConnection;

pub type TestBackend = <TestConnection as Connection>::Backend;

pub fn connection() -> TestConnection {
    let result = connection_without_transaction();
    #[cfg(feature = "sqlite")]
    result.execute("PRAGMA foreign_keys = ON").unwrap();
    result.begin_test_transaction().unwrap();
    result
}

#[cfg(feature = "postgres")]
pub fn connection_without_transaction() -> TestConnection {
    dotenv().ok();
    let connection_url = env::var("PG_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    ::diesel::pg::PgConnection::establish(&connection_url).unwrap()
}

#[cfg(feature = "sqlite")]
embed_migrations!("../migrations/sqlite");

#[cfg(feature = "sqlite")]
pub fn connection_without_transaction() -> TestConnection {
    let connection = ::diesel::sqlite::SqliteConnection::establish(":memory:").unwrap();
    embedded_migrations::run(&connection).unwrap();
    connection
}

#[cfg(feature = "mysql")]
pub fn connection_without_transaction() -> TestConnection {
    dotenv().ok();
    let connection_url = env::var("MYSQL_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    ::diesel::mysql::MysqlConnection::establish(&connection_url).unwrap()
}

sql_function!(nextval, nextval_t, (a: types::VarChar) -> types::BigInt);

pub fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    let connection = connection();
    connection.execute("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .unwrap();
    ensure_primary_key_seq_greater_than(2, &connection);
    connection
}

pub fn connection_with_nullable_table_data() -> TestConnection {
    let connection = connection();

    let test_data = vec![
        NullableColumn { id: 1, value: None },
        NullableColumn { id: 2, value: None },
        NullableColumn { id: 3, value: Some(1) },
        NullableColumn { id: 4, value: Some(2) },
        NullableColumn { id: 5, value: Some(1) },
    ];
    insert(&test_data).into(nullable_table::table)
        .execute(&connection).unwrap();

    connection
}

fn ensure_primary_key_seq_greater_than(x: i64, connection: &TestConnection) {
    if cfg!(feature = "postgres") {
        for _ in 0..x {
            select(nextval("users_id_seq")).execute(connection).unwrap();
        }
    }
}

pub fn find_user_by_name(name: &str, connection: &TestConnection) -> User {
    users::table.filter(users::name.eq(name))
        .first(connection)
        .unwrap()
}

enable_multi_table_joins!(users, comments);
enable_multi_table_joins!(posts, likes);
