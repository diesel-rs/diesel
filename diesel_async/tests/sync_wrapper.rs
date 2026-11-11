use diesel::migration::Migration;
use diesel::{Connection, IntoSql};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;

#[test]
fn test_sync_wrapper() {
    use diesel::RunQueryDsl;

    // The runtime is required for the `sqlite` implementation to be able to use
    // `spawn_blocking()`. This is not required for `postgres` or `mysql`.
    #[cfg(feature = "sqlite")]
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();

    #[cfg(feature = "sqlite")]
    let _guard = rt.enter();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let mut conn = AsyncConnectionWrapper::<crate::TestConnection>::establish(&db_url).unwrap();

    let res =
        diesel::select(1.into_sql::<diesel::sql_types::Integer>()).get_result::<i32>(&mut conn);
    assert_eq!(Ok(1), res);
}

#[tokio::test]
async fn test_sync_wrapper_async_query() {
    use diesel_async::{AsyncConnection, RunQueryDsl};

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let conn = crate::TestConnection::establish(&db_url).await.unwrap();
    let mut conn = AsyncConnectionWrapper::<_>::from(conn);

    let res = diesel::select(1.into_sql::<diesel::sql_types::Integer>())
        .get_result::<i32>(&mut conn)
        .await;
    assert_eq!(Ok(1), res);
}

#[tokio::test]
async fn test_sync_wrapper_under_runtime() {
    use diesel::RunQueryDsl;

    let db_url = std::env::var("DATABASE_URL").unwrap();
    tokio::task::spawn_blocking(move || {
        let mut conn = AsyncConnectionWrapper::<crate::TestConnection>::establish(&db_url).unwrap();

        let res =
            diesel::select(1.into_sql::<diesel::sql_types::Integer>()).get_result::<i32>(&mut conn);
        assert_eq!(Ok(1), res);
    })
    .await
    .unwrap();
}

#[test]
fn check_run_migration() {
    use diesel_migrations::MigrationHarness;

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let migrations: Vec<Box<dyn Migration<crate::TestBackend>>> = Vec::new();
    let mut conn = AsyncConnectionWrapper::<crate::TestConnection>::establish(&db_url).unwrap();

    // just use `run_migrations` here because that's the easiest one without additional setup
    conn.run_migrations(&migrations).unwrap();
}

#[tokio::test]
async fn test_sync_wrapper_unwrap() {
    let db_url = std::env::var("DATABASE_URL").unwrap();

    let conn = tokio::task::spawn_blocking(move || {
        use diesel::RunQueryDsl;

        let mut conn = AsyncConnectionWrapper::<crate::TestConnection>::establish(&db_url).unwrap();
        let res =
            diesel::select(1.into_sql::<diesel::sql_types::Integer>()).get_result::<i32>(&mut conn);
        assert_eq!(Ok(1), res);
        conn
    })
    .await
    .unwrap();

    {
        use diesel_async::RunQueryDsl;

        let mut conn = conn.into_inner();
        let res = diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(&mut conn)
            .await;
        assert_eq!(Ok(1), res);
    }
}
