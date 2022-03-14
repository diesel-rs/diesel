use diesel::prelude::*;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use lazy_static::lazy_static;
use std::sync::{Mutex, MutexGuard};

pub fn connection() -> PgConnection {
    let url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&url).unwrap();
    let migrations = FileBasedMigrations::find_migrations_directory().unwrap();
    conn.run_pending_migrations(migrations).unwrap();
    conn.begin_test_transaction().unwrap();
    conn
}

pub fn this_test_modifies_env() -> MutexGuard<'static, ()> {
    let _ = dotenvy::var("FORCING_DOTENV_LOAD");
    lazy_static! {
        static ref ENV_LOCK: Mutex<()> = Mutex::new(());
    }
    ENV_LOCK.lock().unwrap()
}
