use diesel::*;

infer_schema!(dotenv!("DATABASE_URL"));

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Insertable, AsChangeset, Associations)]
#[has_many(posts)]
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

#[cfg(feature = "postgres")]
#[path="postgres_specific_schema.rs"]
mod backend_specifics;

#[cfg(feature = "sqlite")]
#[path="sqlite_specific_schema.rs"]
mod backend_specifics;

pub use self::backend_specifics::*;

numeric_expr!(users::id);

select_column_workaround!(users -> comments (id, name, hair_color));
select_column_workaround!(comments -> users (id, post_id, text));

join_through!(users -> posts -> comments);

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

#[derive(Insertable)]
#[table_name="comments"]
pub struct NewComment<'a>(
    #[column_name(post_id)]
    pub i32,
    #[column_name(text)]
    pub &'a str,
);

#[cfg(feature = "postgres")]
pub type TestConnection = ::diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
pub type TestConnection = ::diesel::sqlite::SqliteConnection;

pub type TestBackend = <TestConnection as Connection>::Backend;

pub fn connection() -> TestConnection {
    let result = connection_without_transaction();
    result.begin_test_transaction().unwrap();
    result
}

#[cfg(feature = "postgres")]
pub fn connection_without_transaction() -> TestConnection {
    let connection_url = dotenv!("DATABASE_URL",
        "DATABASE_URL must be set in order to run tests");
    ::diesel::pg::PgConnection::establish(&connection_url).unwrap()
}

#[cfg(feature = "sqlite")]
embed_migrations!("../../migrations/sqlite");

#[cfg(feature = "sqlite")]
pub fn connection_without_transaction() -> TestConnection {
    let connection = ::diesel::sqlite::SqliteConnection::establish(":memory:").unwrap();
    embedded_migrations::run(&connection).unwrap();
    connection
}

use diesel::query_builder::insert_statement::{InsertStatement, Insert};
use diesel::query_builder::QueryFragment;

#[cfg(not(feature = "sqlite"))]
pub fn batch_insert<'a, T, U: 'a, Conn>(records: &'a [U], table: T, connection: &Conn)
    -> usize where
        T: Table,
        Conn: Connection,
        &'a [U]: Insertable<T, Conn::Backend>,
        InsertStatement<T, &'a [U], Insert>: QueryFragment<Conn::Backend>,
{
    insert(records).into(table).execute(connection).unwrap()
}

#[cfg(feature = "sqlite")]
pub fn batch_insert<'a, T, U: 'a, Conn>(records: &'a [U], table: T, connection: &Conn)
    -> usize where
        T: Table + Copy,
        Conn: Connection,
        &'a U: Insertable<T, Conn::Backend>,
        InsertStatement<T, &'a U, Insert>: QueryFragment<Conn::Backend>,
{
    for record in records {
        insert(record).into(table).execute(connection).unwrap();
    }
    records.len()
}

sql_function!(nextval, nextval_t, (a: types::VarChar) -> types::BigInt);

pub fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    let connection = connection();
    connection.execute("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .unwrap();
    ensure_primary_key_seq_greater_than(2, &connection);
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
