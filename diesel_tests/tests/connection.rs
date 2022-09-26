use crate::schema::*;
use diesel::connection::BoxableConnection;
use diesel::*;

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn managing_updated_at_for_table() {
    use crate::schema_dsl::*;
    use chrono::NaiveDateTime;
    use std::{thread, time::Duration};

    table! {
        #[sql_name = "auto_time"]
            auto_time_table {
            id -> Integer,
            n -> Integer,
            updated_at -> Timestamp,
        }
    }
    use auto_time_table::dsl::{auto_time_table as auto_time, n, updated_at};

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

#[test]
fn boxable_connection_downcast_mut_usable() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let boxable = connection as &mut dyn BoxableConnection<TestBackend>;
    let connection = boxable.downcast_mut::<TestConnection>().unwrap();
    let sean = users.select(name).find(1).first(connection);

    assert_eq!(Ok(String::from("Sean")), sean);
}

#[test]
#[cfg(feature = "postgres")]
fn use_the_same_connection_multiple_times() {
    use crate::*;
    use diesel::result::DatabaseErrorKind;
    use diesel::result::Error::DatabaseError;

    table! {
        #[sql_name = "github_issue_3342"]
        github_issue_3342_table {
            id -> Serial,
            uid -> Integer,
        }
    }
    use github_issue_3342_table::dsl::{github_issue_3342_table as github_issue_3342, uid};

    let connection = &mut connection_without_transaction();

    // We can extend `schema_dsl` module to accommodate UNIQUE constraint.
    sql_query(
        r#"
          CREATE TABLE github_issue_3342 (
            id SERIAL PRIMARY KEY,
            uid INTEGER NOT NULL UNIQUE
          )
        "#,
    )
    .execute(connection)
    .unwrap();

    let mut _drop_conn = connection_without_transaction();
    let _guard = DropTable {
        connection: &mut _drop_conn,
        table_name: "github_issue_3342",
        can_drop: true,
    };

    // helper method to simulate database error.
    fn insert_or_fetch(conn: &mut PgConnection, input: i32) {
        let result = insert_into(github_issue_3342)
            .values(uid.eq(input))
            .get_result::<(i32, i32)>(conn);

        match result {
            Ok((_, r)) => assert_eq!(r, input),
            Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                let result = github_issue_3342
                    .filter(uid.eq(input))
                    .first::<(i32, i32)>(conn);
                match result {
                    Ok((_, r)) => assert_eq!(r, input),
                    Err(DatabaseError(DatabaseErrorKind::UnableToSendCommand, message))
                        if message.message() == "another command is already in progress\n" =>
                    {
                        panic!("The fix didn't solve the problem!?")
                    }
                    Err(e) => panic!("Caused by: {}", e),
                }
            }
            Err(DatabaseError(DatabaseErrorKind::UnableToSendCommand, message))
                if message.message() == "another command is already in progress\n" =>
            {
                panic!("The fix didn't solve the problem!?")
            }
            Err(e) => panic!("Caused by: {}", e),
        }
    }

    // simulate multiple queries using the same connection sequentially.
    for _ in 0..5 {
        insert_or_fetch(connection, 1);
    }
}
