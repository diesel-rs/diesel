extern crate diesel_migrations;

use self::diesel_migrations::run_pending_migrations;
use diesel::prelude::*;
use dotenv;
use std::sync::{Mutex, MutexGuard};

pub fn connection() -> PgConnection {
    let url = dotenv::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = PgConnection::establish(&url).unwrap();
    run_pending_migrations(&conn).unwrap();
    conn.begin_test_transaction().unwrap();
    conn
}

pub fn this_test_modifies_env() -> MutexGuard<'static, ()> {
    let _ = dotenv::var("FORCING_DOTENV_LOAD");
    lazy_static! {
        static ref ENV_LOCK: Mutex<()> = Mutex::new(());
    }
    ENV_LOCK.lock().unwrap()
}
