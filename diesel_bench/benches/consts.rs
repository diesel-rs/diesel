#![allow(dead_code)]

const INSERT_USERS_PREFIX: &str = "INSERT INTO users (name, hair_color) VALUES ";

pub fn build_insert_users_params(
    size: usize,
    hair_color_init: impl Fn(usize) -> Option<&'static str>,
) -> Vec<(String, Option<&'static str>)> {
    (0..size)
        .map(|x| (format!("User {}", x), hair_color_init(x)))
        .collect()
}

#[cfg(feature = "mysql")]
pub mod mysql {
    use super::*;

    pub const CLEANUP_QUERIES: &[&str] = &[
        "SET FOREIGN_KEY_CHECKS = 0",
        "TRUNCATE TABLE comments",
        "TRUNCATE TABLE posts",
        "TRUNCATE TABLE users",
        "SET FOREIGN_KEY_CHECKS = 1",
    ];

    pub const TRIVIAL_QUERY: &str = "SELECT id, name, hair_color FROM users";

    pub const MEDIUM_COMPLEX_QUERY_BY_ID: &str = "\
        SELECT u.id, u.name, u.hair_color, p.id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = ?";

    pub const MEDIUM_COMPLEX_QUERY_BY_NAME: &str = "\
        SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = ?";

    pub fn build_insert_users_query(size: usize) -> String {
        let mut query = String::from(INSERT_USERS_PREFIX);
        for x in 0..size {
            if x > 0 {
                query.push(',');
            }
            query.push_str("(?,?)");
        }
        query
    }
}

#[cfg(feature = "postgres")]
pub mod postgres {
    use super::*;
    use std::fmt::Write;

    pub const CLEANUP_QUERIES: &[&str] = &[
        "TRUNCATE TABLE comments CASCADE",
        "TRUNCATE TABLE posts CASCADE",
        "TRUNCATE TABLE users CASCADE",
    ];

    pub const TRIVIAL_QUERY: &str = "SELECT id, name, hair_color FROM users";

    pub const MEDIUM_COMPLEX_QUERY_BY_ID: &str = "\
        SELECT u.id, u.name, u.hair_color, p.id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = $1";

    pub const MEDIUM_COMPLEX_QUERY_BY_NAME: &str = "\
        SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = $1";

    pub fn build_insert_users_query(size: usize) -> String {
        let mut query = String::from(INSERT_USERS_PREFIX);
        for x in 0..size {
            if x > 0 {
                query.push(',');
            }
            let idx = x * 2;
            write!(query, "(${},${})", idx + 1, idx + 2).unwrap();
        }
        query
    }
}
