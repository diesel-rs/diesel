#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use schema::translations::{self, dsl};

mod model;
mod schema;

#[derive(Debug, Queryable, Insertable)]
#[table_name = "translations"]
pub struct Translation {
    word_id: i32,
    translation_id: i32,
    language: model::Language,
}

fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));

    let _ = diesel::insert_into(dsl::translations)
        .values(&Translation {
            word_id: 1,
            translation_id: 1,
            language: model::Language::En,
        })
        .execute(&conn);

    let t = dsl::translations
        .select((dsl::word_id, dsl::translation_id, dsl::language))
        .get_results::<Translation>(&conn)
        .expect("select");
    println!("{:?}", t);
}
