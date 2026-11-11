use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

diesel::table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(HasQuery, Debug)]
struct User {
    id: i32,
    name: String,
}

impl User {
    async fn load_all(mut conn: &AsyncPgConnection) -> QueryResult<Vec<Self>> {
        Self::query().load(&mut conn).await
    }

    async fn filter_by_id(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<Option<Self>> {
        Self::query()
            .find(id)
            .get_result(&mut conn)
            .await
            .optional()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL").expect("Env var `DATABASE_URL` not set");
    let conn = AsyncPgConnection::establish(&db_url).await?;

    let all_users = User::query().load(&mut &conn);
    let single_user = User::query().find(1).get_result(&mut &conn);

    let (all_users, single_user) = tokio::try_join!(all_users, single_user)?;
    println!("All users: {all_users:?}");
    println!("Single user: {single_user:?}");

    let (all_users, single_user) =
        tokio::try_join!(User::load_all(&conn), User::filter_by_id(&conn, 1))?;

    println!("All users: {all_users:?}");
    println!("Single user: {single_user:?}");

    Ok(())
}
