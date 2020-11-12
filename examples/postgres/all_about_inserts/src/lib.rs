#[cfg(test)]
use diesel::debug_query;
use diesel::insert_into;
#[cfg(test)]
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::Deserialize;
use std::error::Error;
use std::time::SystemTime;

mod schema {
    diesel::table! {
        users {
            id -> Integer,
            name -> Text,
            hair_color -> Nullable<Text>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }
}

use schema::users;

#[derive(Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct UserForm<'a> {
    name: &'a str,
    hair_color: Option<&'a str>,
}

#[derive(Queryable, PartialEq, Debug)]
struct User {
    id: i32,
    name: String,
    hair_color: Option<String>,
    created_at: SystemTime,
    updated_at: SystemTime,
}

pub fn insert_default_values(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users).default_values().execute(conn)
}

#[test]
fn examine_sql_from_insert_default_values() {
    use schema::users::dsl::*;

    let query = insert_into(users).default_values();
    let sql = "INSERT INTO \"users\" DEFAULT VALUES -- binds: []";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_single_column(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users).values(name.eq("Sean")).execute(conn)
}

#[test]
fn examine_sql_from_insert_single_column() {
    use schema::users::dsl::*;

    let query = insert_into(users).values(name.eq("Sean"));
    let sql = "INSERT INTO \"users\" (\"name\") VALUES ($1) \
               -- binds: [\"Sean\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_multiple_columns(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users)
        .values((name.eq("Tess"), hair_color.eq("Brown")))
        .execute(conn)
}

#[test]
fn examine_sql_from_insert_multiple_columns() {
    use schema::users::dsl::*;

    let query = insert_into(users).values((name.eq("Tess"), hair_color.eq("Brown")));
    let sql = "INSERT INTO \"users\" (\"name\", \"hair_color\") VALUES ($1, $2) \
               -- binds: [\"Tess\", \"Brown\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_insertable_struct(conn: &mut PgConnection) -> Result<(), Box<dyn Error>> {
    use schema::users::dsl::*;

    let json = r#"{ "name": "Sean", "hair_color": "Black" }"#;
    let user_form = serde_json::from_str::<UserForm>(json)?;

    insert_into(users).values(&user_form).execute(conn)?;

    Ok(())
}

#[test]
fn examine_sql_from_insertable_struct() {
    use schema::users::dsl::*;

    let json = r#"{ "name": "Sean", "hair_color": "Black" }"#;
    let user_form = serde_json::from_str::<UserForm>(json).unwrap();
    let query = insert_into(users).values(&user_form);
    let sql = "INSERT INTO \"users\" (\"name\", \"hair_color\") VALUES ($1, $2) \
               -- binds: [\"Sean\", \"Black\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_insertable_struct_option(conn: &mut PgConnection) -> Result<(), Box<dyn Error>> {
    use schema::users::dsl::*;

    let json = r#"{ "name": "Ruby", "hair_color": null }"#;
    let user_form = serde_json::from_str::<UserForm>(json)?;

    insert_into(users).values(&user_form).execute(conn)?;

    Ok(())
}

#[test]
fn examine_sql_from_insertable_struct_option() {
    use schema::users::dsl::*;

    let json = r#"{ "name": "Ruby", "hair_color": null }"#;
    let user_form = serde_json::from_str::<UserForm>(json).unwrap();
    let query = insert_into(users).values(&user_form);
    let sql = "INSERT INTO \"users\" (\"name\", \"hair_color\") VALUES ($1, DEFAULT) \
               -- binds: [\"Ruby\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_single_column_batch(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users)
        .values(&vec![name.eq("Sean"), name.eq("Tess")])
        .execute(conn)
}

#[test]
fn examine_sql_from_insert_single_column_batch() {
    use schema::users::dsl::*;

    let values = vec![name.eq("Sean"), name.eq("Tess")];
    let query = insert_into(users).values(&values);
    let sql = "INSERT INTO \"users\" (\"name\") VALUES ($1), ($2) \
               -- binds: [\"Sean\", \"Tess\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_single_column_batch_with_default(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users)
        .values(&vec![Some(name.eq("Sean")), None])
        .execute(conn)
}

#[test]
fn examine_sql_from_insert_single_column_batch_with_default() {
    use schema::users::dsl::*;

    let values = vec![Some(name.eq("Sean")), None];
    let query = insert_into(users).values(&values);
    let sql = "INSERT INTO \"users\" (\"name\") VALUES ($1), (DEFAULT) \
               -- binds: [\"Sean\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_tuple_batch(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users)
        .values(&vec![
            (name.eq("Sean"), hair_color.eq("Black")),
            (name.eq("Tess"), hair_color.eq("Brown")),
        ])
        .execute(conn)
}

#[test]
fn examine_sql_from_insert_tuple_batch() {
    use schema::users::dsl::*;

    let values = vec![
        (name.eq("Sean"), hair_color.eq("Black")),
        (name.eq("Tess"), hair_color.eq("Brown")),
    ];
    let query = insert_into(users).values(&values);
    let sql = "INSERT INTO \"users\" (\"name\", \"hair_color\") \
               VALUES ($1, $2), ($3, $4) \
               -- binds: [\"Sean\", \"Black\", \"Tess\", \"Brown\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_tuple_batch_with_default(conn: &mut PgConnection) -> QueryResult<usize> {
    use schema::users::dsl::*;

    insert_into(users)
        .values(&vec![
            (name.eq("Sean"), Some(hair_color.eq("Black"))),
            (name.eq("Ruby"), None),
        ])
        .execute(conn)
}

#[test]
fn examine_sql_from_insert_tuple_batch_with_default() {
    use schema::users::dsl::*;

    let values = vec![
        (name.eq("Sean"), Some(hair_color.eq("Black"))),
        (name.eq("Ruby"), None),
    ];
    let query = insert_into(users).values(&values);
    let sql = "INSERT INTO \"users\" (\"name\", \"hair_color\") \
               VALUES ($1, $2), ($3, DEFAULT) \
               -- binds: [\"Sean\", \"Black\", \"Ruby\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn insert_insertable_struct_batch(conn: &mut PgConnection) -> Result<(), Box<dyn Error>> {
    use schema::users::dsl::*;

    let json = r#"[
        { "name": "Sean", "hair_color": "Black" },
        { "name": "Tess", "hair_color": "Brown" }
    ]"#;
    let user_form = serde_json::from_str::<Vec<UserForm>>(json)?;

    insert_into(users).values(&user_form).execute(conn)?;

    Ok(())
}

#[test]
fn examine_sql_from_insertable_struct_batch() {
    use schema::users::dsl::*;

    let json = r#"[
        { "name": "Sean", "hair_color": "Black" },
        { "name": "Tess", "hair_color": "Brown" }
    ]"#;
    let user_form = serde_json::from_str::<Vec<UserForm>>(json).unwrap();
    let query = insert_into(users).values(&user_form);
    let sql = "INSERT INTO \"users\" (\"name\", \"hair_color\") \
               VALUES ($1, $2), ($3, $4) \
               -- binds: [\"Sean\", \"Black\", \"Tess\", \"Brown\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

#[test]
fn insert_get_results_batch() {
    let conn = &mut establish_connection();
    conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
        use diesel::select;
        use schema::users::dsl::*;

        let now = select(diesel::dsl::now).get_result::<SystemTime>(conn)?;

        let inserted_users = insert_into(users)
            .values(&vec![
                (id.eq(1), name.eq("Sean")),
                (id.eq(2), name.eq("Tess")),
            ])
            .get_results(conn)?;

        let expected_users = vec![
            User {
                id: 1,
                name: "Sean".into(),
                hair_color: None,
                created_at: now,
                updated_at: now,
            },
            User {
                id: 2,
                name: "Tess".into(),
                hair_color: None,
                created_at: now,
                updated_at: now,
            },
        ];
        assert_eq!(expected_users, inserted_users);

        Ok(())
    });
}

#[test]
fn examine_sql_from_insert_get_results_batch() {
    use diesel::query_builder::AsQuery;
    use schema::users::dsl::*;

    let values = vec![(id.eq(1), name.eq("Sean")), (id.eq(2), name.eq("Tess"))];
    let query = insert_into(users).values(&values).as_query();
    let sql = "INSERT INTO \"users\" (\"id\", \"name\") VALUES ($1, $2), ($3, $4) \
               RETURNING \"users\".\"id\", \"users\".\"name\", \
               \"users\".\"hair_color\", \"users\".\"created_at\", \
               \"users\".\"updated_at\" -- binds: [1, \"Sean\", 2, \"Tess\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

#[test]
fn insert_get_result() {
    let conn = &mut establish_connection();
    conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
        use diesel::select;
        use schema::users::dsl::*;

        let now = select(diesel::dsl::now).get_result::<SystemTime>(conn)?;

        let inserted_user = insert_into(users)
            .values((id.eq(3), name.eq("Ruby")))
            .get_result(conn)?;

        let expected_user = User {
            id: 3,
            name: "Ruby".into(),
            hair_color: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(expected_user, inserted_user);

        Ok(())
    });
}

#[test]
fn examine_sql_from_insert_get_result() {
    use diesel::query_builder::AsQuery;
    use schema::users::dsl::*;

    let query = insert_into(users)
        .values((id.eq(3), name.eq("Ruby")))
        .as_query();
    let sql = "INSERT INTO \"users\" (\"id\", \"name\") VALUES ($1, $2) \
               RETURNING \"users\".\"id\", \"users\".\"name\", \
               \"users\".\"hair_color\", \"users\".\"created_at\", \
               \"users\".\"updated_at\" -- binds: [3, \"Ruby\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

pub fn explicit_returning(conn: &mut PgConnection) -> QueryResult<i32> {
    use schema::users::dsl::*;

    insert_into(users)
        .values(name.eq("Ruby"))
        .returning(id)
        .get_result(conn)
}

#[test]
fn examine_sql_from_explicit_returning() {
    use schema::users::dsl::*;

    let query = insert_into(users).values(name.eq("Ruby")).returning(id);
    let sql = "INSERT INTO \"users\" (\"name\") VALUES ($1) \
               RETURNING \"users\".\"id\" \
               -- binds: [\"Ruby\"]";
    assert_eq!(sql, debug_query::<Pg, _>(&query).to_string());
}

#[cfg(test)]
fn establish_connection() -> PgConnection {
    let url = ::std::env::var("DATABASE_URL").unwrap();
    PgConnection::establish(&url).unwrap()
}
