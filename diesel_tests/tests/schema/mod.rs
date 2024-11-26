use diesel::*;

#[cfg(feature = "postgres")]
mod custom_schemas;

#[cfg(feature = "postgres")]
include!("pg_schema.rs");
#[cfg(feature = "sqlite")]
include!("sqlite_schema.rs");
#[cfg(feature = "mysql")]
include!("mysql_schema.rs");

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
    QueryableByName,
    Selectable,
)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

impl User {
    pub fn new(id: i32, name: &str) -> Self {
        User {
            id,
            name: name.to_string(),
            hair_color: None,
        }
    }

    pub fn with_hair_color(id: i32, name: &str, hair_color: &str) -> Self {
        User {
            id,
            name: name.to_string(),
            hair_color: Some(hair_color.to_string()),
        }
    }

    pub fn new_post(&self, title: &str, body: Option<&str>) -> NewPost {
        NewPost::new(self.id, title, body)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = users)]
pub struct UserName(#[diesel(column_name = name)] pub String);

impl UserName {
    pub fn new(name: &str) -> Self {
        UserName(name.to_string())
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations, Selectable)]
#[diesel(belongs_to(Post), table_name = comments)]
pub struct Comment {
    id: i32,
    post_id: i32,
    text: String,
}

impl Comment {
    pub fn new(id: i32, post_id: i32, text: &str) -> Self {
        Comment {
            id,
            post_id,
            text: text.into(),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Queryable, Insertable, Associations, Identifiable, Selectable,
)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Post))]
#[diesel(table_name = followings)]
#[diesel(primary_key(user_id, post_id))]
pub struct Following {
    pub user_id: i32,
    pub post_id: i32,
    pub email_notifications: bool,
}

#[rustfmt::skip]
#[cfg_attr(feature = "postgres", path = "postgres_specific_schema.rs")]
#[cfg_attr(not(feature = "postgres"), path = "backend_specifics.rs")]
mod backend_specifics;

pub use self::backend_specifics::*;

#[derive(Debug, PartialEq, Eq, Queryable, Clone, Insertable, AsChangeset, Selectable)]
#[diesel(table_name = users)]
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

#[derive(Debug, PartialEq, Eq, Insertable)]
#[diesel(table_name = users)]
pub struct DefaultColorUser {
    pub name: String,
    pub hair_color: Option<Option<String>>,
}

impl DefaultColorUser {
    pub fn new(name: &str, hair_color: Option<Option<&str>>) -> Self {
        Self {
            name: name.to_string(),
            hair_color: hair_color.map(|o| o.map(|s| s.to_string())),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = posts)]
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

#[derive(Debug, Clone, Copy, Insertable)]
#[diesel(table_name = comments)]
pub struct NewComment<'a>(
    #[diesel(column_name = post_id)] pub i32,
    #[diesel(column_name = text)] pub &'a str,
);

#[derive(PartialEq, Eq, Debug, Clone, Insertable)]
#[diesel(table_name = fk_tests)]
pub struct FkTest {
    id: i32,
    fk_id: i32,
}

impl FkTest {
    pub fn new(id: i32, fk_id: i32) -> Self {
        FkTest { id, fk_id }
    }
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = nullable_table)]
pub struct NullableColumn {
    id: i32,
    value: Option<i32>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Queryable, Insertable, Identifiable, Associations, Selectable,
)]
#[diesel(table_name = likes)]
#[diesel(primary_key(comment_id, user_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Comment))]
pub struct Like {
    pub comment_id: i32,
    pub user_id: i32,
}

#[cfg(feature = "postgres")]
pub type TestConnection = PgConnection;
#[cfg(feature = "sqlite")]
pub type TestConnection = SqliteConnection;
#[cfg(feature = "mysql")]
pub type TestConnection = MysqlConnection;

pub type TestBackend = <TestConnection as Connection>::Backend;

//Used to ensure cleanup of one-off tables, e.g. for a table created for a single test
pub struct DropTable<'a> {
    pub connection: &'a mut TestConnection,
    pub table_name: &'static str,
    pub can_drop: bool,
}

impl Drop for DropTable<'_> {
    fn drop(&mut self) {
        if self.can_drop {
            diesel::sql_query(format!("DROP TABLE {}", self.table_name))
                .execute(self.connection)
                .unwrap();
        }
    }
}

#[cfg(feature = "sqlite")]
const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("../migrations/sqlite");

#[cfg(feature = "postgres")]
const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("../migrations/postgres");

#[cfg(feature = "mysql")]
const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("../migrations/mysql");

pub fn connection() -> TestConnection {
    let mut result = connection_without_transaction();
    result.begin_test_transaction().unwrap();
    result
}

pub fn connection_without_transaction() -> TestConnection {
    use diesel_migrations::MigrationHarness;
    use std::sync::Once;
    static MIGRATION_GUARD: Once = Once::new();

    let mut result = backend_specific_connection();

    if cfg!(feature = "sqlite") {
        result.run_pending_migrations(MIGRATIONS).unwrap();
    } else {
        MIGRATION_GUARD.call_once(|| {
            result.run_pending_migrations(MIGRATIONS).unwrap();
        });
    }

    result
}

#[cfg(feature = "postgres")]
pub fn backend_specific_connection() -> TestConnection {
    let connection_url = dotenvy::var("PG_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    let mut conn = PgConnection::establish(&connection_url).unwrap();

    // we do match the error messages in some tests and depending on your
    // operating system configuration postgres may return localized error messages
    // This forces the language to english
    diesel::sql_query("SET lc_messages TO 'en_US.UTF-8'")
        .execute(&mut conn)
        .unwrap();
    conn
}

#[cfg(feature = "sqlite")]
pub fn backend_specific_connection() -> TestConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    diesel::sql_query("PRAGMA foreign_keys = ON")
        .execute(&mut conn)
        .unwrap();
    conn
}

#[cfg(feature = "mysql")]
pub fn backend_specific_connection() -> TestConnection {
    let connection_url = dotenvy::var("MYSQL_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests");
    MysqlConnection::establish(&connection_url).unwrap()
}

#[cfg(feature = "postgres")]
pub fn disable_foreign_keys(connection: &mut TestConnection) {
    diesel::sql_query("SET CONSTRAINTS ALL DEFERRED")
        .execute(connection)
        .unwrap();
}

#[cfg(feature = "mysql")]
pub fn disable_foreign_keys(connection: &mut TestConnection) {
    diesel::sql_query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(connection)
        .unwrap();
}

#[cfg(feature = "sqlite")]
pub fn disable_foreign_keys(connection: &mut TestConnection) {
    diesel::sql_query("PRAGMA defer_foreign_keys = ON")
        .execute(connection)
        .unwrap();
}

#[cfg(feature = "sqlite")]
pub fn drop_table_cascade(connection: &mut TestConnection, table: &str) {
    diesel::sql_query(format!("DROP TABLE {table}"))
        .execute(connection)
        .unwrap();
}

#[cfg(feature = "postgres")]
pub fn drop_table_cascade(connection: &mut TestConnection, table: &str) {
    diesel::sql_query(format!("DROP TABLE {table} CASCADE"))
        .execute(connection)
        .unwrap();
}

#[declare_sql_function]
extern "SQL" {
    fn nextval(a: sql_types::VarChar) -> sql_types::BigInt;
}

pub fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    let mut connection = connection();
    insert_sean_and_tess_into_users_table(&mut connection);
    connection
}

pub fn insert_sean_and_tess_into_users_table(connection: &mut TestConnection) {
    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .execute(connection)
        .unwrap();
    ensure_primary_key_seq_greater_than(2, connection);
}

pub fn connection_with_gilbert_and_jonathan_in_users_table() -> TestConnection {
    let mut connection = connection();
    insert_gilbert_and_jonathan_into_users_table(&mut connection);
    connection
}

pub fn insert_gilbert_and_jonathan_into_users_table(connection: &mut TestConnection) {
    diesel::sql_query("INSERT INTO users (id, name, hair_color) VALUES (1, 'Gilbert', 'brown'), (2, 'Jonathan', 'electric-blue')")
        .execute(connection)
        .unwrap();
    ensure_primary_key_seq_greater_than(2, connection);
}

pub fn connection_with_nullable_table_data() -> TestConnection {
    let mut connection = connection();

    let test_data = vec![
        NullableColumn { id: 1, value: None },
        NullableColumn { id: 2, value: None },
        NullableColumn {
            id: 3,
            value: Some(1),
        },
        NullableColumn {
            id: 4,
            value: Some(2),
        },
        NullableColumn {
            id: 5,
            value: Some(1),
        },
    ];
    insert_into(nullable_table::table)
        .values(&test_data)
        .execute(&mut connection)
        .unwrap();

    connection
}

fn ensure_primary_key_seq_greater_than(x: i64, connection: &mut TestConnection) {
    if cfg!(feature = "postgres") {
        for _ in 0..x {
            select(nextval("users_id_seq")).execute(connection).unwrap();
        }
    }
}

pub fn find_user_by_name(name: &str, connection: &mut TestConnection) -> User {
    users::table
        .filter(users::name.eq(name))
        .first(connection)
        .unwrap()
}
