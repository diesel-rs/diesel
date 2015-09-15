#[derive(PartialEq, Eq, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

impl User {
    pub fn new(id: i32, name: &str) -> Self {
        User { id: id, name: name.to_string(), hair_color: None }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
}

// Compiler plugin will automatically invoke this based on schema
table! {
    users {
        id -> Serial,
        name -> VarChar,
        hair_color -> Nullable<VarChar>,
    }
}

table! {
    posts {
        id -> Serial,
        user_id -> Integer,
        title -> VarChar,
    }
}

// Compiler plugin will replace this with #[derive(Queriable)]
queriable! {
    User {
        id -> i32,
        name -> String,
        hair_color -> Option<String>,
    }
}

queriable! {
    Post {
        id -> i32,
        user_id -> i32,
        title -> String,
    }
}

joinable!(posts -> users (user_id = id));
belongs_to!(User, users, Post, posts);

#[derive(Debug, PartialEq, Eq)]
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

insertable! {
    NewUser -> users {
        name -> String,
        hair_color -> Option<String>,
    }
}

queriable! {
    NewUser {
        name -> String,
        hair_color -> Option<String>,
    }
}

use Connection;

pub fn setup_users_table(connection: &Connection) {
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR
    )").unwrap();
}

pub fn setup_posts_table(connection: &Connection) {
    connection.execute("CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        user_id INTEGER NOT NULL,
        title VARCHAR NOT NULL
    )").unwrap();
}

pub fn connection() -> Connection {
    let connection_url = ::std::env::var("DATABASE_URL").ok()
        .expect("DATABASE_URL must be set in order to run tests");
    let result = Connection::establish(&connection_url).unwrap();
    result.execute("BEGIN").unwrap();
    result
}
