use diesel::*;

#[derive(PartialEq, Eq, Debug, Clone, Queryable)]
#[changeset_for(users)]
#[has_many(posts)]
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

#[derive(PartialEq, Eq, Debug, Clone, Queryable)]
#[has_many(comments)]
#[belongs_to(user)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
    pub tags: Vec<String>,
}

impl Post {
    pub fn new(id: i32, user_id: i32, title: &str, body: Option<&str>) -> Self {
        Post {
            id: id,
            user_id: user_id,
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
            tags: Vec::new(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable)]
pub struct Comment {
    id: i32,
    post_id: i32,
    text: String,
}

infer_schema!(dotenv!("DATABASE_URL"));
numeric_expr!(users::id);

select_column_workaround!(users -> comments (id, name, hair_color));
select_column_workaround!(comments -> users (id, post_id, text));

join_through!(users -> posts -> comments);

#[derive(Debug, PartialEq, Eq, Queryable)]
#[insertable_into(users)]
#[changeset_for(users)]
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

#[insertable_into(posts)]
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

#[insertable_into(comments)]
pub struct NewComment<'a>(
    #[column_name="post_id"]
    pub i32,
    #[column_name="text"]
    pub &'a str,
);

#[cfg(feature = "postgres")]
pub type TestConnection = ::diesel::connection::PgConnection;
#[cfg(feature = "sqlite")]
pub type TestConnection = ::diesel::connection::SqliteConnection;

pub fn connection() -> TestConnection {
    let result = connection_without_transaction();
    result.begin_test_transaction().unwrap();
    result
}

#[cfg(feature = "postgres")]
pub fn connection_without_transaction() -> TestConnection {
    let connection_url = dotenv!("DATABASE_URL",
        "DATABASE_URL must be set in order to run tests");
    ::diesel::connection::PgConnection::establish(&connection_url).unwrap()
}

#[cfg(feature = "sqlite")]
pub fn connection_without_transaction() -> TestConnection {
    let connection = ::diesel::connection::SqliteConnection::establish(":memory:").unwrap();
    let migrations_dir = migrations::find_migrations_directory().unwrap().join("sqlite");
    migrations::run_pending_migrations_in_directory(&connection, &migrations_dir).unwrap();
    connection
}


pub fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    let connection = connection();
    connection.execute("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .unwrap();
    connection
}

pub fn find_user_by_name(name: &str, connection: &TestConnection) -> User {
    users::table.filter(users::name.eq(name))
        .first(connection)
        .unwrap()
}
