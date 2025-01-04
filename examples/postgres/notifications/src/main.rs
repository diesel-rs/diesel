use diesel::sql_query;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {database_url}"))
}

fn main() {
    let conn = &mut establish_connection();

    sql_query("LISTEN example_channel").execute(conn).unwrap();
    sql_query("NOTIFY example_channel, 'additional data'").execute(conn).unwrap();

    let mut iter = conn.notifications_iter();
    let notification = iter.next().unwrap();

    assert_eq!(notification.channel, "example_channel");
    assert_eq!(notification.payload, "additional data");
    println!("This process id: {}", std::process::id());
    println!("Notification received from server process with id {}.", notification.process_id);
}
