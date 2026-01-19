use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

use crate::schema::TestConnection;
use diesel::QueryResult;
use diesel_migrations::{MigrationHarness, RustMigration};
use diesel_migrations::{RustMigrationSource, TypedMigration};

static GLOBAL_FLAG: AtomicU16 = AtomicU16::new(0);

fn function(_conn: &mut TestConnection) -> QueryResult<()> {
    GLOBAL_FLAG.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

struct Typed(Arc<AtomicBool>);

impl TypedMigration<TestConnection> for Typed {
    fn up(&self, _conn: &mut TestConnection) -> QueryResult<()> {
        self.0.store(true, Ordering::Relaxed);
        Ok(())
    }
}

#[diesel_test_helper::test]
fn test_rust_migrations() {
    let mut source = RustMigrationSource::<TestConnection>::new();
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called2 = callback_called.clone();
    let typed_called = Arc::new(AtomicBool::new(false));
    let rust_migration_called = Arc::new(AtomicBool::new(false));
    let rust_migration_called2 = rust_migration_called.clone();

    source
        .add_migration(
            "2026-01-23-173320_test1",
            move |_conn: &mut TestConnection| {
                callback_called.store(true, Ordering::Relaxed);
                Ok(())
            },
        )
        .unwrap();
    source
        .add_migration("2026-01-23-173920_test2", function)
        .unwrap();
    source
        .add_migration("2026-01-23-174320_test3", Typed(typed_called.clone()))
        .unwrap();
    let migration = RustMigration::new(move |_conn: &mut TestConnection| {
        rust_migration_called.store(true, Ordering::Relaxed);
        Ok(())
    })
    .with_down(|_| Ok(()))
    .without_transaction();

    source
        .add_migration("2026-01-23-174620_test4", migration)
        .unwrap();

    #[cfg(not(feature = "mysql"))]
    let conn = &mut crate::schema::connection();
    #[cfg(feature = "mysql")]
    let conn = &mut crate::schema::connection_without_transaction();

    assert!(!typed_called.load(Ordering::Relaxed));
    assert!(!callback_called2.load(Ordering::Relaxed));
    assert!(!rust_migration_called2.load(Ordering::Relaxed));
    assert_eq!(GLOBAL_FLAG.load(Ordering::Relaxed), 0);
    let res = conn.run_pending_migrations(source.clone()).map(|c| c.len());
    if cfg!(feature = "mysql") {
        let _ = conn.revert_all_migrations(source);
    }

    let res = res.unwrap();
    assert_eq!(res, 4);

    assert!(typed_called.load(Ordering::Relaxed));
    assert!(callback_called2.load(Ordering::Relaxed));
    assert!(rust_migration_called2.load(Ordering::Relaxed));
    assert_eq!(GLOBAL_FLAG.load(Ordering::Relaxed), 1);
}
