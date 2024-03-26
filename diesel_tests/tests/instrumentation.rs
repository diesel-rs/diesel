use crate::schema::users;
use crate::schema::TestConnection;
use diesel::connection::DefaultLoadingMode;
use diesel::connection::InstrumentationEvent;
use diesel::connection::LoadConnection;
use diesel::connection::SimpleConnection;
use diesel::query_builder::AsQuery;
use diesel::Connection;
use diesel::QueryResult;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::Mutex;

use crate::schema::connection_with_sean_and_tess_in_users_table;

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

fn setup_test_case() -> (Arc<Mutex<Vec<Event>>>, TestConnection) {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let events_to_check = events.clone();
    let mut conn = connection_with_sean_and_tess_in_users_table();
    conn.set_instrumentation(move |event: InstrumentationEvent<'_>| {
        events.lock().unwrap().push(event.into());
    });
    assert_eq!(events_to_check.lock().unwrap().len(), 0);
    (events_to_check, conn)
}

#[test]
fn check_events_are_emitted_for_batch_execute() {
    let (events_to_check, mut conn) = setup_test_case();
    conn.batch_execute("select 1").unwrap();

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

#[test]
fn check_events_are_emitted_for_execute_returning_count() {
    let (events_to_check, mut conn) = setup_test_case();
    conn.execute_returning_count(&users::table.as_query())
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 3, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
}

#[test]
fn check_events_are_emitted_for_load() {
    let (events_to_check, mut conn) = setup_test_case();
    LoadConnection::<DefaultLoadingMode>::load(&mut conn, users::table.as_query()).unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 3, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
}

#[test]
fn check_events_are_emitted_for_execute_returning_count_does_not_contain_cache_for_uncached_queries(
) {
    let (events_to_check, mut conn) = setup_test_case();
    conn.execute_returning_count(&diesel::sql_query("select 1"))
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { .. });
}

#[test]
fn check_events_are_emitted_for_load_does_not_contain_cache_for_uncached_queries() {
    let (events_to_check, mut conn) = setup_test_case();
    LoadConnection::<DefaultLoadingMode>::load(&mut conn, diesel::sql_query("select 1")).unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { .. });
}

#[test]
fn check_events_are_emitted_for_execute_returning_count_does_contain_error_for_failures() {
    let (events_to_check, mut conn) = setup_test_case();
    let _ = conn.execute_returning_count(&diesel::sql_query("invalid"));
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: Some(_), .. });
}

#[test]
fn check_events_are_emitted_for_load_does_contain_error_for_failures() {
    let (events_to_check, mut conn) = setup_test_case();
    let _ = LoadConnection::<DefaultLoadingMode>::load(&mut conn, diesel::sql_query("invalid"));
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: Some(_), .. });
}

#[test]
fn check_events_are_emitted_for_execute_returning_count_repeat_does_not_repeat_cache() {
    let (events_to_check, mut conn) = setup_test_case();
    conn.execute_returning_count(&users::table.as_query())
        .unwrap();
    conn.execute_returning_count(&users::table.as_query())
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 5, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::StartQuery { .. });
    assert_matches!(events[4], Event::FinishQuery { .. });
}

#[test]
fn check_events_are_emitted_for_load_repeat_does_not_repeat_cache() {
    let (events_to_check, mut conn) = setup_test_case();
    LoadConnection::<DefaultLoadingMode>::load(&mut conn, users::table.as_query()).unwrap();
    LoadConnection::<DefaultLoadingMode>::load(&mut conn, users::table.as_query()).unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 5, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::StartQuery { .. });
    assert_matches!(events[4], Event::FinishQuery { .. });
}

#[test]
fn check_events_transaction() {
    let (events_to_check, mut conn) = setup_test_case();
    conn.transaction(|_conn| QueryResult::Ok(())).unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 6, "{:?}", events);
    assert_matches!(events[0], Event::BeginTransaction { .. });
    assert_matches!(events[1], Event::StartQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::CommitTransaction { .. });
    assert_matches!(events[4], Event::StartQuery { .. });
    assert_matches!(events[5], Event::FinishQuery { .. });
}

#[test]
fn check_events_transaction_error() {
    let (events_to_check, mut conn) = setup_test_case();
    let _ = conn
        .transaction(|_conn| QueryResult::<()>::Err(diesel::result::Error::RollbackTransaction));
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 6, "{:?}", events);
    assert_matches!(events[0], Event::BeginTransaction { .. });
    assert_matches!(events[1], Event::StartQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
    assert_matches!(events[3], Event::RollbackTransaction { .. });
    assert_matches!(events[4], Event::StartQuery { .. });
    assert_matches!(events[5], Event::FinishQuery { .. });
}

#[test]
fn check_events_transaction_nested() {
    let (events_to_check, mut conn) = setup_test_case();
    conn.transaction(|conn| conn.transaction(|_conn| QueryResult::Ok(())))
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 12, "{:?}", events);
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
#[test]
fn check_events_are_emitted_for_load_pg_row_by_row() {
    use diesel::pg::PgRowByRowLoadingMode;

    let (events_to_check, mut conn) = setup_test_case();
    LoadConnection::<PgRowByRowLoadingMode>::load(&mut conn, users::table.as_query()).unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 3, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::CacheQuery { .. });
    assert_matches!(events[2], Event::FinishQuery { .. });
}

#[cfg(feature = "postgres")]
#[test]
fn check_events_are_emitted_for_load_does_not_contain_cache_for_uncached_queries_pg_row_by_row() {
    use diesel::pg::PgRowByRowLoadingMode;

    let (events_to_check, mut conn) = setup_test_case();
    LoadConnection::<PgRowByRowLoadingMode>::load(&mut conn, diesel::sql_query("select 1"))
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { .. });
}

#[cfg(feature = "postgres")]
#[test]
fn check_events_are_emitted_for_load_does_contain_error_for_failures_pg_row_by_row() {
    use diesel::pg::PgRowByRowLoadingMode;

    let (events_to_check, mut conn) = setup_test_case();
    let _ = LoadConnection::<PgRowByRowLoadingMode>::load(&mut conn, diesel::sql_query("invalid"));
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: Some(_), .. });
}

#[cfg(feature = "postgres")]
#[test]
fn check_events_are_emitted_for_copy_to() {
    use diesel::pg::CopyFormat;
    use diesel::ExecuteCopyFromDsl;

    let (events_to_check, mut conn) = setup_test_case();

    let _count = diesel::copy_from(users::table)
        .from_raw_data(users::table, |copy| {
            writeln!(copy, "3,Sean,").unwrap();
            writeln!(copy, "4,Tess,").unwrap();
            diesel::QueryResult::Ok(())
        })
        .with_format(CopyFormat::Csv)
        .execute(&mut conn)
        .unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: None, .. });
}

#[cfg(feature = "postgres")]
#[test]
fn check_events_are_emitted_for_copy_to_with_error() {
    use diesel::pg::CopyFormat;
    use diesel::ExecuteCopyFromDsl;

    let (events_to_check, mut conn) = setup_test_case();

    let count = diesel::copy_from(users::table)
        .from_raw_data(users::table, |_copy| {
            diesel::QueryResult::Err(diesel::result::Error::RollbackTransaction)
        })
        .with_format(CopyFormat::Csv)
        .execute(&mut conn);
    assert!(count.is_err());
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: Some(_), .. });
}

#[cfg(feature = "postgres")]
#[test]
fn check_events_are_emitted_for_copy_from() {
    use diesel::pg::CopyFormat;
    use std::io::Read;

    let (events_to_check, mut conn) = setup_test_case();

    let mut out = String::new();
    let mut copy = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .load_raw(&mut conn)
        .unwrap();
    copy.read_to_string(&mut out).unwrap();
    let events = events_to_check.lock().unwrap();
    assert_eq!(events.len(), 2, "{:?}", events);
    assert_matches!(events[0], Event::StartQuery { .. });
    assert_matches!(events[1], Event::FinishQuery { error: None, .. });
}
