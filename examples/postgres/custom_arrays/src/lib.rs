use diesel::r2d2::R2D2Connection;
use diesel::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::error::Error;

pub mod model;
mod schema;

// Alias for a pooled connection.
// pub type Connection = diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>;

// Alias for a normal, single, connection.
pub type Connection = PgConnection;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Runs all pending database migrations.
///
/// Will return an error if the database connection is invalid, or if any of the
/// migrations fail. Otherwise, it returns Ok()
///
/// # Errors
///
/// * If the database connection is invalid
/// * If checking for pending database migrations fails
/// * If any of the database migrations fail
///
pub fn run_db_migration(
    conn: &mut Connection,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // Check DB connection!
    match conn.ping() {
        Ok(_) => {}
        Err(e) => {
            eprint!("[run_db_migration]: Error connecting to database: {}", e);
            return Err(Box::new(e));
        }
    }

    // Check if DB has pending migrations
    let has_pending = match conn.has_pending_migration(MIGRATIONS) {
        Ok(has_pending) => has_pending,
        Err(e) => {
            eprint!(
                "[run_db_migration]: Error checking for pending database migrations: {}",
                e
            );
            return Err(e);
        }
    };

    // If so, run all pending migrations.
    if has_pending {
        match conn.run_pending_migrations(MIGRATIONS) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprint!("[run_db_migration]: Error migrating database: {}", e);
                Err(e)
            }
        }
    } else {
        // Nothing pending, just return
        Ok(())
    }
}
