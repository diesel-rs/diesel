use super::schema::*;
use diesel::*;
use schema_dsl::*;

#[test]
fn union_on_selects() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data =
        if cfg!(feature = "postgres") {
            vec![
                "Tess".to_string(),
                "Sean".to_string(),
            ]
        } else if cfg!(feature = "sqlite") {
            vec![
                "Sean".to_string(),
                "Tess".to_string(),
            ]
        } else {
            vec![]
        };
    let query = users.select(name);
    let union = query.union(query);
    let actual_data: Vec<String> = union
        .load(&connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn union_all_on_selects() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data = vec![
        "Sean".to_string(),
        "Tess".to_string(),
        "Sean".to_string(),
        "Tess".to_string(),
     ];
    let query = users.select(name);
    let union = query.union_all(query);
    let actual_data: Vec<String> = union
        .load(&connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn union_on_unions() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data =
        if cfg!(feature = "postgres") {
            vec![
                "Tess".to_string(),
                "Sean".to_string(),
            ]
        } else if cfg!(feature = "sqlite") {
            vec![
                "Sean".to_string(),
                "Tess".to_string(),
            ]
        } else {
            vec![]
        };
    let query = users.select(name);
    let union = query.union(query).union(query);
    let actual_data: Vec<String> = union
        .load(&connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}
