use diesel::prelude::*;
use diesel::sql_query;
use dotenvy::dotenv;
use std::env;
use std::error::Error;

fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {database_url}"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let conn = &mut establish_connection();

    sql_query("LISTEN example_channel").execute(conn)?;
    sql_query("NOTIFY example_channel, 'additional data'").execute(conn)?;

    for result in conn.notifications_iter() {
        let notification = result.unwrap();
        assert_eq!(notification.channel, "example_channel");
        assert_eq!(notification.payload, "additional data");

        println!(
            "Notification received from server process with id {}.",
            notification.process_id
        );
    }
    Ok(())
}
