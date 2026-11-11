use crate::users;
use crate::TestConnection;
use assert_matches::assert_matches;
use diesel::connection::InstrumentationEvent;
use diesel::query_builder::AsQuery;
use diesel::QueryResult;
use diesel_async::AsyncConnection;
use diesel_async::AsyncConnectionCore;
use diesel_async::SimpleAsyncConnection;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::Mutex;

async fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    super::connection().await
}

#[derive(Debug, PartialEq)]
enum Event {
    StartQuery { query: String },
    CacheQuery { sql: String },
    FinishQuery { query: String, error: Option<()> },
    BeginTransaction { depth: NonZeroU32 },
    CommitTransaction { depth: NonZeroU32 },
    RollbackTransaction { depth: NonZeroU32 },
}

impl From<InstrumentationEvent<'_>> for Event {
    fn from(value: InstrumentationEvent<'_>) -> Self {
        match value {
            InstrumentationEvent::StartEstablishConnection { .. } => unreachable!(),
            InstrumentationEvent::FinishEstablishConnection { .. } => unreachable!(),
            InstrumentationEvent::StartQuery { query, .. } => Event::StartQuery {
                query: query.to_string(),
            },
            InstrumentationEvent::CacheQuery { sql, .. } => Event::CacheQuery {
                sql: sql.to_owned(),
            },
            InstrumentationEvent::FinishQuery { query, error, .. } => Event::FinishQuery {
                query: query.to_string(),
                error: error.map(|_| ()),
            },
            InstrumentationEvent::BeginTransaction { depth, .. } => {
                Event::BeginTransaction { depth }
            }
            InstrumentationEvent::CommitTransaction { depth, .. } => {
                Event::CommitTransaction { depth }
            }
            InstrumentationEvent::RollbackTransaction { depth, .. } => {
                Event::RollbackTransaction { depth }
            }
            _ => unreachable!(),
        }
    }
}

async fn setup_test_case() -> (Arc<Mutex<Vec<Event>>>, TestConnection) {
    setup_test_case_with_connection(connection_with_sean_and_tess_in_users_table().await)
}

fn setup_test_case_with_connection(
    mut conn: TestConnection,
) -> (Arc<Mutex<Vec<Event>>>, TestConnection) {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let events_to_check = events.clone();
    conn.set_instrumentation(move |event: InstrumentationEvent<'_>| {
        events.lock().unwrap().push(event.into());
    });
    assert_eq!(events_to_check.lock().unwrap().len(), 0);
    (events_to_check, conn)
}

#[tokio::test]
async fn check_events_are_emitted_for_batch_execute() {
    let (events_to_check, mut conn) = setup_test_case().await;
    conn.batch_execute("select 1").await.unwrap();

    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        Event::StartQuery {
            query: String::from("select 1")
        }
    );
    assert_eq!(
        events[1],
        Event::FinishQuery {
            query: String::from("select 1"),
            error: None,
        }
    );
}

#[tokio::test]
async fn check_events_are_emitted_for_execute_returning_count() {
    let (events_to_check, mut conn) = setup_test_case().await;
    conn.execute_returning_count(users::table.as_query())
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 3, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_load() {
    let (events_to_check, mut conn) = setup_test_case().await;
    let _ = AsyncConnectionCore::load(&mut conn, users::table.as_query())
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 3, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_execute_returning_count_does_not_contain_cache_for_uncached_queries(
) {
    let (events_to_check, mut conn) = setup_test_case().await;
    conn.execute_returning_count(diesel::sql_query("select 1"))
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_load_does_not_contain_cache_for_uncached_queries() {
    let (events_to_check, mut conn) = setup_test_case().await;
    let _ = AsyncConnectionCore::load(&mut conn, diesel::sql_query("select 1"))
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_execute_returning_count_does_contain_error_for_failures() {
    let (events_to_check, mut conn) = setup_test_case().await;
    let _ = conn
        .execute_returning_count(diesel::sql_query("invalid"))
        .await;
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: Some(_), .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_load_does_contain_error_for_failures() {
    let (events_to_check, mut conn) = setup_test_case().await;
    let _ = AsyncConnectionCore::load(&mut conn, diesel::sql_query("invalid")).await;
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: Some(_), .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_execute_returning_count_repeat_does_not_repeat_cache() {
    let (events_to_check, mut conn) = setup_test_case().await;
    conn.execute_returning_count(users::table.as_query())
        .await
        .unwrap();
    conn.execute_returning_count(users::table.as_query())
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 5, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::StartQuery { .. });
    assert_matches!(events[4], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_are_emitted_for_load_repeat_does_not_repeat_cache() {
    let (events_to_check, mut conn) = setup_test_case().await;
    let _ = AsyncConnectionCore::load(&mut conn, users::table.as_query())
        .await
        .unwrap();
    let _ = AsyncConnectionCore::load(&mut conn, users::table.as_query())
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 5, "{events:?}");
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::StartQuery { .. });
    assert_matches!(events[4], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_transaction() {
    let (events_to_check, mut conn) = setup_test_case().await;
    conn.transaction(async |_conn| QueryResult::Ok(()))
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 6, "{events:?}");
    assert_matches!(events[0], Event::BeginTransaction { .. });
    assert_matches!(events[1], Event::StartQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::CommitTransaction { .. });
    assert_matches!(events[4], Event::StartQuery { .. });
    assert_matches!(events[5], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_transaction_error() {
    let (events_to_check, mut conn) = setup_test_case().await;
    let _ = conn
        .transaction(async |_conn| {
            QueryResult::<()>::Err(diesel::result::Error::RollbackTransaction)
        })
        .await;
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 6, "{events:?}");
    assert_matches!(events[0], Event::BeginTransaction { .. });
    assert_matches!(events[1], Event::StartQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::RollbackTransaction { .. });
    assert_matches!(events[4], Event::StartQuery { .. });
    assert_matches!(events[5], Event::FinishQuery { .. });
}

#[tokio::test]
async fn check_events_transaction_nested() {
    let (events_to_check, mut conn) = setup_test_case().await;
    conn.transaction(async |conn| conn.transaction(async |_conn| QueryResult::Ok(())).await)
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 12, "{events:?}");
    assert_matches!(events[0], Event::BeginTransaction { .. });
    assert_matches!(events[1], Event::StartQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::BeginTransaction { .. });
    assert_matches!(events[4], Event::StartQuery { .. });
    assert_matches!(events[5], Event::FinishQuery { .. });
    assert_matches!(events[6], Event::CommitTransaction { .. });
    assert_matches!(events[7], Event::StartQuery { .. });
    assert_matches!(events[8], Event::FinishQuery { .. });
    assert_matches!(events[9], Event::CommitTransaction { .. });
    assert_matches!(events[10], Event::StartQuery { .. });
    assert_matches!(events[11], Event::FinishQuery { .. });
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn check_events_transaction_builder() {
    use crate::connection_without_transaction;
    use diesel::result::Error;

    let (events_to_check, mut conn) =
        setup_test_case_with_connection(connection_without_transaction().await);
    conn.build_transaction()
        .run(async |_tx| Ok::<(), Error>(()))
        .await
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 6, "{events:?}");
    assert_matches!(events[0], Event::BeginTransaction { .. });
    assert_matches!(events[1], Event::StartQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::CommitTransaction { .. });
    assert_matches!(events[4], Event::StartQuery { .. });
    assert_matches!(events[5], Event::FinishQuery { .. });
}
