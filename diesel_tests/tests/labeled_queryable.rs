use super::schema::*;
use diesel::connection::SimpleConnection;
use diesel::*;
use schema_dsl::*;

use diesel::deserialize::Field;
use diesel::deserialize::NamedQueryable;
use diesel::frunk::labelled::chars::*;
use diesel::frunk::{HCons, HNil};
use diesel::query_dsl::load_dsl::labelled_query;

#[derive(Debug, PartialEq, NamedQueryable)]
struct ReorderedUser {
    name: String,
    id: i32,
    hair_color: Option<String>,
}

#[test]
fn selecting_basic_data() {
    use schema::users::dsl::*;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let actual_data: Vec<ReorderedUser> =
        labelled_query(users.select((hair_color, id, name)), &connection).unwrap();

    let expected_data = vec![
        ReorderedUser {
            name: "Sean".into(),
            id: 1,
            hair_color: None,
        },
        ReorderedUser {
            name: "Tess".into(),
            id: 2,
            hair_color: None,
        },
    ];

    assert_eq!(expected_data, actual_data);
}

#[derive(Debug, PartialEq, NamedQueryable)]
struct SingleFieldUser {
    name: String,
}

#[test]
fn selecting_single_field_data() {
    use schema::users::dsl::*;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let actual_data: Vec<SingleFieldUser> =
        labelled_query(users.select(name), &connection).unwrap();

    let expected_data = vec![
        SingleFieldUser {
            name: "Sean".into(),
        },
        SingleFieldUser {
            name: "Tess".into(),
        },
    ];

    assert_eq!(expected_data, actual_data);
}
