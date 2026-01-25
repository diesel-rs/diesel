pub mod mysql {
    pub const CLEANUP_QUERIES: &[&str] = &[
        "SET FOREIGN_KEY_CHECKS = 0",
        "TRUNCATE TABLE comments",
        "TRUNCATE TABLE posts",
        "TRUNCATE TABLE users",
        "SET FOREIGN_KEY_CHECKS = 1",
    ];

    pub const MEDIUM_COMPLEX_QUERY_BY_ID: &str = "\
        SELECT u.id, u.name, u.hair_color, p.id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = ?";

    pub const MEDIUM_COMPLEX_QUERY_BY_NAME: &str = "\
        SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = ?";
}

pub mod postgres {
    pub const CLEANUP_QUERIES: &[&str] = &[
        "TRUNCATE TABLE comments CASCADE",
        "TRUNCATE TABLE posts CASCADE",
        "TRUNCATE TABLE users CASCADE",
    ];

    pub const MEDIUM_COMPLEX_QUERY_BY_ID: &str = "\
        SELECT u.id, u.name, u.hair_color, p.id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = $1";

    pub const MEDIUM_COMPLEX_QUERY_BY_NAME: &str = "\
        SELECT u.id as myuser_id, u.name, u.hair_color, p.id as post_id, p.user_id, p.title, p.body \
        FROM users as u LEFT JOIN posts as p on u.id = p.user_id WHERE u.hair_color = $1";
}
