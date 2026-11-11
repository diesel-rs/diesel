use diesel::prelude::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel::QueryResult;
use diesel_async::*;
use std::fmt::Debug;

#[cfg(feature = "postgres")]
mod custom_types;
#[cfg(feature = "postgres")]
mod errors;
mod instrumentation;
#[cfg(feature = "migrations")]
mod migrations;
mod multiconnection;
mod notifications;
#[cfg(any(feature = "bb8", feature = "deadpool", feature = "mobc"))]
mod pooling;
mod run_query_dsl_impl_match;
#[cfg(feature = "async-connection-wrapper")]
mod sync_wrapper;
mod transactions;
mod type_check;

async fn transaction_test<C: AsyncConnection<Backend = TestBackend>>(
    conn: &mut C,
) -> QueryResult<()> {
    let res = conn
        .transaction::<i32, diesel::result::Error, _>(async |conn| {
            let users: Vec<User> = users::table.load(conn).await?;
            assert_eq!(&users[0].name, "John Doe");
            assert_eq!(&users[1].name, "Jane Doe");

            let user: Option<User> = users::table.find(42).first(conn).await.optional()?;
            assert_eq!(user, None::<User>);

            let res = conn
                .transaction::<_, diesel::result::Error, _>(async |conn| {
                    diesel::insert_into(users::table)
                        .values(users::name.eq("Dave"))
                        .execute(conn)
                        .await?;
                    let count = users::table.count().get_result::<i64>(conn).await?;
                    assert_eq!(count, 3);
                    Ok(())
                })
                .await;
            assert!(res.is_ok());
            let count = users::table.count().get_result::<i64>(conn).await?;
            assert_eq!(count, 3);

            let res = diesel::insert_into(users::table)
                .values(users::name.eq("Eve"))
                .execute(conn)
                .await?;

            assert_eq!(res, 1, "Insert in transaction returned wrong result");
            let count = users::table.count().get_result::<i64>(conn).await?;
            assert_eq!(count, 4);

            Err(diesel::result::Error::RollbackTransaction)
        })
        .await;
    assert_eq!(
        res,
        Err(diesel::result::Error::RollbackTransaction),
        "Failed to rollback transaction"
    );

    let count = users::table.count().get_result::<i64>(conn).await?;
    assert_eq!(count, 2, "user got committed, but transaction rolled back");

    Ok(())
}

diesel::table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(
    diesel::Queryable,
    diesel::Selectable,
    Debug,
    PartialEq,
    diesel::AsChangeset,
    diesel::Identifiable,
)]
struct User {
    id: i32,
    name: String,
}

#[cfg(feature = "mysql")]
type TestConnection = AsyncMysqlConnection;
#[cfg(feature = "mariadb")]
type TestConnection = AsyncMariadbConnection;
#[cfg(feature = "postgres")]
type TestConnection = AsyncPgConnection;
#[cfg(feature = "sqlite")]
type TestConnection =
    sync_connection_wrapper::SyncConnectionWrapper<diesel::sqlite::SqliteConnection>;

#[allow(dead_code)]
type TestBackend = <TestConnection as AsyncConnectionCore>::Backend;

#[tokio::test]
async fn test_basic_insert_and_load() -> QueryResult<()> {
    let conn = &mut connection().await;
    // Insertion split into 2 since Sqlite batch insert isn't supported for diesel_async yet
    let res = diesel::insert_into(users::table)
        .values(users::name.eq("John Doe"))
        .execute(conn)
        .await;
    assert_eq!(res, Ok(1), "User count does not match");
    let res = diesel::insert_into(users::table)
        .values(users::name.eq("Jane Doe"))
        .execute(conn)
        .await;
    assert_eq!(res, Ok(1), "User count does not match");
    let users = users::table.load::<User>(conn).await?;
    assert_eq!(&users[0].name, "John Doe", "User name [0] does not match");
    assert_eq!(&users[1].name, "Jane Doe", "User name [1] does not match");

    transaction_test(conn).await?;

    Ok(())
}

// The sqlite wrapper hands diesel a `CollectedQuery`, which diesel caches by its SQL
#[cfg(any(feature = "postgres", feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn cached_statement_with_a_different_bind_count_is_rejected() {
    use diesel::query_builder::{AstPass, Query, QueryFragment, QueryId};
    use diesel::sql_types::{Bool, Integer};

    // The derived `QueryId` claims a static query id although the SQL depends on
    // the number of values, so both queries below share one statement cache entry
    #[derive(QueryId)]
    struct InList(Vec<i32>);

    impl QueryFragment<TestBackend> for InList {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, TestBackend>) -> QueryResult<()> {
            out.push_sql("SELECT 1 IN (");
            for (i, v) in self.0.iter().enumerate() {
                if i > 0 {
                    out.push_sql(", ");
                }
                out.push_bind_param::<Integer, _>(v)?;
            }
            out.push_sql(")");
            Ok(())
        }
    }

    impl Query for InList {
        type SqlType = Bool;
    }

    impl diesel::query_dsl::RunQueryDslSupport for InList {}

    let conn = &mut connection().await;
    let r = InList(vec![2]).get_result::<bool>(conn).await;
    assert_eq!(r, Ok(false));
    let r = InList((1..=64).collect()).get_result::<bool>(conn).await;
    assert_eq!(
        r.map_err(|e| e.to_string()),
        Err(
            "prepared statement expected a different number of parameters than provided: \
             Expected 1, provided: 64"
                .to_owned()
        )
    );
}

#[cfg(feature = "postgres")]
diesel::define_sql_function!(fn pg_sleep(interval: diesel::sql_types::Double));

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres_cancel_token() {
    use std::time::Duration;

    use diesel::result::{DatabaseErrorKind, Error};

    let mut conn = connection().await;

    let token = conn.cancel_token();

    // execute a query that runs for a long time
    let long_running_query = async move {
        diesel::select(pg_sleep(5.0))
            .execute(&mut conn)
            .await
            .expect_err("query should have been canceled.")
    };

    // execute the query elsewhere...
    let task = tokio::spawn(long_running_query);

    // let the task above have some time to actually start...
    tokio::time::sleep(Duration::from_millis(500)).await;

    // invoke the cancellation token.
    token.cancel_query(tokio_postgres::NoTls).await.unwrap();

    // make sure the query task resulted in a cancellation error
    let err = task.await.unwrap();
    match err {
        Error::DatabaseError(DatabaseErrorKind::Unknown, v)
            if v.message() == "canceling statement due to user request" => {}
        _ => panic!("unexpected error: {err:?}"),
    }
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn mysql_cancel_token() {
    use diesel::result::{DatabaseErrorKind, Error};
    use std::time::Duration;

    let (sender, receiver) = tokio::sync::oneshot::channel();

    // execute a long-running query in a separate future
    let query_future = async move {
        let conn = &mut connection().await;
        let token = conn.cancel_token();

        // send the token back to the main thread via a oneshot channel
        sender
            .send(token)
            .unwrap_or_else(|_| panic!("couldn't send token"));

        diesel::dsl::sql::<diesel::sql_types::Integer>("SELECT SLEEP(5)")
            .get_result::<i32>(conn)
            .await
    };
    let cancel_future = async move {
        // wait for the cancellation token to be sent
        if let Ok(token) = receiver.await {
            // give the query time to start before invoking the token
            tokio::time::sleep(Duration::from_millis(500)).await;
            token.cancel_query().await.unwrap();
        } else {
            panic!("Failed to receive cancel token");
        }
    };

    let (task, _) = tokio::join!(query_future, cancel_future);

    // make sure the query task resulted in a cancellation error or a return value of 1:
    match task {
        Err(Error::DatabaseError(DatabaseErrorKind::Unknown, v))
            if v.message() == "Query execution was interrupted" => {}
        Err(e) => panic!("unexpected error: {:?}", e),
        // mysql 8.4 returns 1 from a canceled sleep instead of an error
        Ok(1) => {}
        Ok(_) => panic!("query completed successfully without cancellation"),
    }
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn mysql_sqlstate_reports_the_code_of_a_mapped_error() {
    use diesel::result::{DatabaseErrorKind, Error};

    let conn = &mut connection().await;
    let user = (users::id.eq(1), users::name.eq("Sean"));

    diesel::insert_into(users::table)
        .values(user)
        .execute(conn)
        .await
        .unwrap();
    let err = diesel::insert_into(users::table)
        .values(user)
        .execute(conn)
        .await
        .unwrap_err();

    let Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info) = err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(info.sqlstate().unwrap().as_str(), "23000");
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn mysql_sqlstate_reports_a_custom_code_raised_by_signal() {
    use diesel::result::{DatabaseErrorKind, Error};

    let conn = &mut connection().await;

    // MySQL rejects `SIGNAL` as a prepared statement, so it goes through the text protocol.
    let err = conn
        .batch_execute("SIGNAL SQLSTATE 'P1001' SET MESSAGE_TEXT = 'custom'")
        .await
        .unwrap_err();

    let Error::DatabaseError(DatabaseErrorKind::Unknown, info) = err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(info.sqlstate().unwrap().as_str(), "P1001");
    assert_eq!(info.message(), "custom");
}

#[cfg(feature = "sync-connection-wrapper")]
#[tokio::test(flavor = "multi_thread")]
async fn cancel_blocking_task() {
    let mut conn = connection().await;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let future = conn.spawn_blocking(|_| {
        tx.send(()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    });
    rx.await.unwrap();
    std::mem::drop(future);
    conn.transaction_state();
}

#[cfg(feature = "postgres")]
async fn setup(connection: &mut TestConnection) {
    diesel::sql_query(
        "CREATE TEMPORARY TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR NOT NULL
            )",
    )
    .execute(connection)
    .await
    .unwrap();
}

#[cfg(feature = "sqlite")]
async fn setup(connection: &mut TestConnection) {
    diesel::sql_query(
        "CREATE TEMPORARY TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )",
    )
    .execute(connection)
    .await
    .unwrap();
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
async fn setup(connection: &mut TestConnection) {
    diesel::sql_query(
        "CREATE TEMPORARY TABLE users (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                name TEXT NOT NULL
            ) CHARACTER SET utf8mb4",
    )
    .execute(connection)
    .await
    .unwrap();
}

async fn connection() -> TestConnection {
    let mut conn = connection_without_transaction().await;
    if cfg!(feature = "postgres") {
        // postgres allows to modify the schema inside of a transaction
        conn.begin_test_transaction().await.unwrap();
    }
    setup(&mut conn).await;
    if cfg!(feature = "mysql") || cfg!(feature = "mariadb") || cfg!(feature = "sqlite") {
        // mysql does not allow this and does even automatically close
        // any open transaction. As of this we open a transaction **after**
        // we setup the schema
        conn.begin_test_transaction().await.unwrap();
    }
    conn
}

async fn connection_without_transaction() -> TestConnection {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    TestConnection::establish(&db_url).await.unwrap()
}
