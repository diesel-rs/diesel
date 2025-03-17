//! Example: using COPY FROM and COPY TO with dynamic tables
//!
//! To run this, set DATABASE_URL to a PostgreSQL connection string:
//!
//! ```sh
//! DATABASE_URL=postgres://localhost/diesel_test \
//!     cargo run -p diesel-dynamic-schema --example copy_to_from \
//!     --features "diesel-dynamic-schema/postgres,diesel/postgres"
//! ```
extern crate diesel;
extern crate diesel_dynamic_schema;

use diesel::pg::CopyFormat;
use diesel::prelude::*;
use diesel_dynamic_schema::table;
use std::io::Read;

fn main() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run this example");
    let conn = &mut PgConnection::establish(&database_url).unwrap();

    // Create a temporary table using raw SQL
    diesel::sql_query(
        "CREATE TEMP TABLE copy_example (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
    )
    .execute(conn)
    .unwrap();

    // Use diesel-dynamic-schema to reference the table at runtime
    let copy_example = table("copy_example");

    // Use COPY FROM to insert data in CSV format
    let count = diesel::copy_from(copy_example)
        .from_raw_data(copy_example, |copy| {
            writeln!(copy, "1,Alice").unwrap();
            writeln!(copy, "2,Bob").unwrap();
            writeln!(copy, "3,Charlie").unwrap();
            diesel::QueryResult::Ok(())
        })
        .with_format(CopyFormat::Csv)
        .execute(conn)
        .unwrap();

    println!("Inserted {count} rows via COPY FROM");

    // Use COPY TO to read the data back in CSV format
    let mut copy = diesel::copy_to(copy_example)
        .with_format(CopyFormat::Csv)
        .load_raw(conn)
        .unwrap();

    let mut buf = String::new();
    copy.read_to_string(&mut buf).unwrap();

    println!("COPY TO output:");
    print!("{buf}");
}
