use super::{users, User};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
#[cfg(not(feature = "sqlite"))]
use diesel_async::SaveChangesDsl;

#[tokio::test]
#[cfg(feature = "bb8")]
async fn save_changes_bb8() {
    use diesel_async::pooled_connection::bb8::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;

    let db_url = std::env::var("DATABASE_URL").unwrap();

    let config = AsyncDieselConnectionManager::<super::TestConnection>::new(db_url);
    let pool = Pool::builder().max_size(1).build(config).await.unwrap();

    let mut conn = pool.get().await.unwrap();

    super::setup(&mut conn).await;

    diesel::insert_into(users::table)
        .values(users::name.eq("John"))
        .execute(&mut conn)
        .await
        .unwrap();

    let u = users::table.first::<User>(&mut conn).await.unwrap();
    assert_eq!(u.name, "John");

    #[cfg(not(feature = "sqlite"))]
    {
        let mut u = u;
        u.name = "Jane".into();
        let u2: User = u.save_changes(&mut conn).await.unwrap();

        assert_eq!(u2.name, "Jane");
    }
}

#[tokio::test]
#[cfg(feature = "deadpool")]
async fn save_changes_deadpool() {
    use diesel_async::pooled_connection::deadpool::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;

    let db_url = std::env::var("DATABASE_URL").unwrap();

    let config = AsyncDieselConnectionManager::<super::TestConnection>::new(db_url);
    let pool = Pool::builder(config).max_size(1).build().unwrap();

    let mut conn = pool.get().await.unwrap();

    super::setup(&mut conn).await;

    diesel::insert_into(users::table)
        .values(users::name.eq("John"))
        .execute(&mut conn)
        .await
        .unwrap();

    let u = users::table.first::<User>(&mut conn).await.unwrap();
    assert_eq!(u.name, "John");

    #[cfg(not(feature = "sqlite"))]
    {
        let mut u = u;
        u.name = "Jane".into();
        let u2: User = u.save_changes(&mut conn).await.unwrap();

        assert_eq!(u2.name, "Jane");
    }
}

#[tokio::test]
#[cfg(feature = "mobc")]
async fn save_changes_mobc() {
    use diesel_async::pooled_connection::mobc::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;

    let db_url = std::env::var("DATABASE_URL").unwrap();

    let config = AsyncDieselConnectionManager::<super::TestConnection>::new(db_url);
    let pool = Pool::new(config);

    let mut conn = pool.get().await.unwrap();

    super::setup(&mut conn).await;

    diesel::insert_into(users::table)
        .values(users::name.eq("John"))
        .execute(&mut conn)
        .await
        .unwrap();

    let u = users::table.first::<User>(&mut conn).await.unwrap();
    assert_eq!(u.name, "John");

    #[cfg(not(feature = "sqlite"))]
    {
        let mut u = u;
        u.name = "Jane".into();
        let u2: User = u.save_changes(&mut conn).await.unwrap();

        assert_eq!(u2.name, "Jane");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(all(feature = "sync-connection-wrapper", feature = "deadpool"))]
async fn cancel_blocking_task_deadpool() {
    use diesel_async::pooled_connection::deadpool::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use tokio::sync::oneshot;

    let db_url = std::env::var("DATABASE_URL").unwrap();

    let config = AsyncDieselConnectionManager::<super::TestConnection>::new(db_url);
    let pool = Pool::builder(config).max_size(1).build().unwrap();
    let pool_clone = pool.clone();
    let (tx, rx) = oneshot::channel();

    let handle = tokio::spawn(async move {
        let mut conn = pool_clone.get().await.unwrap();
        conn.spawn_blocking(|_| {
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        })
        .await
        .unwrap();
    });

    rx.await.unwrap();
    handle.abort();
    let _conn = pool.get().await.unwrap();
}

/// `CAST(1 + SLEEP(2) AS SIGNED)`: still running 100 ms after it was sent.
#[cfg(any(feature = "mysql", feature = "mariadb"))]
fn slow_query() -> diesel::expression::SqlLiteral<diesel::sql_types::Integer> {
    diesel::dsl::sql("CAST(1 + SLEEP(2) AS SIGNED)")
}

/// A query dropped while waiting for its result (here by a timeout) leaves the
/// result on its way to the connection, so the connection must report itself
/// as broken instead of being reused.
#[tokio::test]
#[cfg(all(
    any(feature = "bb8", feature = "deadpool", feature = "mobc"),
    any(feature = "mysql", feature = "mariadb")
))]
async fn connection_with_cancelled_query_is_broken() {
    use diesel_async::pooled_connection::PoolableConnection;
    use std::time::Duration;

    let mut conn = super::connection_without_transaction().await;
    assert!(!conn.is_broken());
    let cancelled = tokio::time::timeout(
        Duration::from_millis(100),
        diesel::select(slow_query()).get_result::<i32>(&mut conn),
    )
    .await;
    assert!(
        cancelled.is_err(),
        "the slow query should still be running when it is dropped"
    );
    assert!(conn.is_broken());
}

/// Without a checkout test the next borrower's query is the first to read from
/// the connection, so reusing it would hand that query the stale result.
#[tokio::test]
#[cfg(all(feature = "bb8", any(feature = "mysql", feature = "mariadb")))]
async fn cancelled_query_does_not_poison_bb8() {
    use diesel_async::pooled_connection::bb8::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use std::time::Duration;

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let config = AsyncDieselConnectionManager::<super::TestConnection>::new(db_url);
    let pool = Pool::builder()
        .max_size(1)
        .test_on_check_out(false)
        .build(config)
        .await
        .unwrap();

    // Acquiring the connection must not consume the query's timeout budget.
    let mut conn = pool.get().await.unwrap();
    let before = pool.state().statistics;
    let cancelled = tokio::time::timeout(
        Duration::from_millis(100),
        diesel::select(slow_query()).get_result::<i32>(&mut conn),
    )
    .await;
    assert!(
        cancelled.is_err(),
        "the slow query should still be running when it is dropped"
    );
    drop(conn);

    for n in [12345, 67890] {
        let mut conn = pool.get().await.unwrap();
        let value = diesel::select(n.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(&mut conn)
            .await;
        assert_eq!(value, Ok(n));
    }

    let after = pool.state().statistics;
    assert_eq!(
        after.connections_closed_broken,
        before.connections_closed_broken + 1
    );
    assert_eq!(after.connections_created, before.connections_created + 1);
}
