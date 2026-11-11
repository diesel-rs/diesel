use diesel::prelude::*;
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::{AsyncConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

// ordinary diesel model setup

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[allow(dead_code)]
#[derive(Debug, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type InnerConnection = SqliteConnection;

type InnerDB = Sqlite;

async fn establish(db_url: &str) -> ConnectionResult<SyncConnectionWrapper<InnerConnection>> {
    // It is necessary to specify the specific inner connection type because of inference issues
    SyncConnectionWrapper::<SqliteConnection>::establish(db_url).await
}

async fn run_migrations<A>(async_connection: A) -> Result<(), Box<dyn std::error::Error>>
where
    A: AsyncConnection<Backend = InnerDB> + 'static,
{
    let mut async_wrapper: AsyncConnectionWrapper<A> =
        AsyncConnectionWrapper::from(async_connection);

    tokio::task::spawn_blocking(move || {
        async_wrapper.run_pending_migrations(MIGRATIONS).unwrap();
    })
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

async fn transaction(
    async_conn: &mut SyncConnectionWrapper<InnerConnection>,
    old_name: &str,
    new_name: &str,
) -> Result<Vec<User>, diesel::result::Error> {
    async_conn
        .transaction::<Vec<User>, diesel::result::Error, _>(async |c| {
            if old_name.is_empty() {
                Ok(Vec::new())
            } else {
                diesel::update(users::table.filter(users::name.eq(old_name)))
                    .set(users::name.eq(new_name))
                    .load(c)
                    .await
            }
        })
        .await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL").expect("Env var `DATABASE_URL` not set");

    // create an async connection for the migrations
    let sync_wrapper: SyncConnectionWrapper<InnerConnection> = establish(&db_url).await?;
    run_migrations(sync_wrapper).await?;

    let mut sync_wrapper: SyncConnectionWrapper<InnerConnection> = establish(&db_url).await?;

    diesel::delete(users::table)
        .execute(&mut sync_wrapper)
        .await?;

    diesel::insert_into(users::table)
        .values((users::id.eq(3), users::name.eq("toto")))
        .execute(&mut sync_wrapper)
        .await?;

    let data: Vec<User> = users::table
        .select(User::as_select())
        .load(&mut sync_wrapper)
        .await?;
    println!("{data:?}");

    diesel::delete(users::table)
        .execute(&mut sync_wrapper)
        .await?;

    diesel::insert_into(users::table)
        .values((users::id.eq(1), users::name.eq("iLuke")))
        .execute(&mut sync_wrapper)
        .await?;

    let data: Vec<User> = users::table
        .filter(users::id.gt(0))
        .or_filter(users::name.like("%Luke"))
        .select(User::as_select())
        .load(&mut sync_wrapper)
        .await?;
    println!("{data:?}");

    // a quick test to check if we correctly handle transactions
    let mut conn_a: SyncConnectionWrapper<InnerConnection> = establish(&db_url).await?;
    let mut conn_b: SyncConnectionWrapper<InnerConnection> = establish(&db_url).await?;

    let handle_1 = tokio::spawn(async move {
        loop {
            let changed = transaction(&mut conn_a, "iLuke", "JustLuke").await;
            println!("Changed {changed:?}");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    let handle_2 = tokio::spawn(async move {
        loop {
            let changed = transaction(&mut conn_b, "JustLuke", "iLuke").await;
            println!("Changed {changed:?}");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    let _ = handle_2.await;
    let _ = handle_1.await;

    Ok(())
}
