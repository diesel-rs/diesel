use diesel_async::AsyncMigrationHarness;
use diesel_migrations::MigrationHarness;

static SEQUENTIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

// These two tests are mostly smoke tests to verify
// that the `AsyncMigrationHarness` actually implements
// the necessary traits

#[tokio::test(flavor = "multi_thread")]
async fn plain_connection() {
    let _guard = SEQUENTIAL.lock().unwrap();
    let conn = super::connection().await;
    let mut harness = AsyncMigrationHarness::from(conn);
    harness.applied_migrations().unwrap();
}

#[cfg(feature = "deadpool")]
#[tokio::test(flavor = "multi_thread")]
async fn pool_connection() {
    use diesel_async::pooled_connection::deadpool::Pool;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    let _guard = SEQUENTIAL.lock().unwrap();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let config = AsyncDieselConnectionManager::<super::TestConnection>::new(db_url);
    let pool = Pool::builder(config).build().unwrap();
    let conn = pool.get().await.unwrap();
    let mut harness = AsyncMigrationHarness::from(conn);
    harness.applied_migrations().unwrap();
}
