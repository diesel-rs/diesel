#![cfg(not(feature = "mysql"))]
use crate::schema::{connection_without_transaction, DropTable};
use diesel::*;

table! {
    auto_time {
        id -> Integer,
        n -> Integer,
        updated_at -> Timestamp,
    }
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn managing_updated_at_for_table() {
    use self::auto_time::dsl::*;
    use crate::schema_dsl::*;
    use chrono::NaiveDateTime;
    use std::{thread, time::Duration};

    // transactions have frozen time, so we can't use them
    let connection = &mut connection_without_transaction();
    create_table(
        "auto_time",
        (
            integer("id").primary_key().auto_increment(),
            integer("n"),
            timestamp("updated_at"),
        ),
    )
    .execute(connection)
    .unwrap();
    let mut _drop_conn = connection_without_transaction();
    let _guard = DropTable {
        connection: &mut _drop_conn,
        table_name: "auto_time",
        can_drop: !cfg!(feature = "sqlite"),
    };

    sql_query("SELECT diesel_manage_updated_at('auto_time')")
        .execute(connection)
        .unwrap();

    insert_into(auto_time)
        .values(&vec![n.eq(2), n.eq(1), n.eq(5)])
        .execute(connection)
        .unwrap();

    let result = auto_time
        .count()
        .filter(updated_at.is_null())
        .get_result::<i64>(connection);
    assert_eq!(Ok(3), result);

    update(auto_time)
        .set(n.eq(n + 1))
        .execute(connection)
        .unwrap();

    let result = auto_time
        .count()
        .filter(updated_at.is_null())
        .get_result::<i64>(connection);
    assert_eq!(Ok(0), result);

    if cfg!(feature = "sqlite") {
        // SQLite only has second precision
        thread::sleep(Duration::from_millis(1000));
    }

    let query = auto_time.find(2).select(updated_at);
    let old_time: NaiveDateTime = query.first(connection).unwrap();
    update(auto_time.find(2))
        .set(n.eq(0))
        .execute(connection)
        .unwrap();
    let new_time: NaiveDateTime = query.first(connection).unwrap();
    assert!(old_time < new_time);
}

#[test]
#[cfg(feature = "sqlite")]
fn strips_sqlite_url_prefix() {
    let mut path = std::env::temp_dir();
    path.push("diesel_test_sqlite.db");
    assert!(SqliteConnection::establish(&format!("sqlite://{}", path.display())).is_ok());
}

#[test]
#[cfg(feature = "sqlite")]
fn file_uri_created_in_memory() {
    use std::path::Path;

    assert!(SqliteConnection::establish("file::memory:").is_ok());
    assert!(!Path::new("file::memory:").exists());
    assert!(!Path::new(":memory:").exists());
}

#[test]
#[cfg(feature = "sqlite")]
fn sqlite_uri_prefix_interpreted_as_file() {
    let mut path = std::env::temp_dir();
    path.push("diesel_test_sqlite_readonly.db");
    assert!(SqliteConnection::establish(&format!("sqlite://{}?mode=rwc", path.display())).is_ok());
    assert!(path.exists());
}
