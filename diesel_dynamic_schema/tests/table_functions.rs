//! Table-valued functions declared at run time through `diesel_dynamic_schema`.

#![cfg(feature = "sqlite")]

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::sqlite::{Sqlite, SqliteFunctionBehavior};
use diesel_dynamic_schema::dynamic_value::{Any, DynamicRow, SqliteAnyValue};
use diesel_dynamic_schema::{table, DynamicSelectClause};

fn establish_connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

fn every_storage_class() -> Vec<SqliteAnyValue> {
    vec![
        SqliteAnyValue::Null,
        SqliteAnyValue::Integer(i64::MIN),
        SqliteAnyValue::Real(0.5),
        SqliteAnyValue::Text("text".into()),
        SqliteAnyValue::Blob(vec![0, 159, 255]),
    ]
}

fn register_grantees(conn: &mut SqliteConnection) -> QueryResult<()> {
    let grantees = table("grantees");
    conn.register_table_valued_function(
        vec![grantees.column::<Any, _>("name")],
        vec![grantees.column::<Any, _>("role")],
        SqliteFunctionBehavior::INNOCUOUS,
        |conn, arguments: DynamicRow<SqliteAnyValue>| {
            arguments[0]
                .bind_to(
                    diesel::sql_query("SELECT name FROM grants WHERE role = ?1 ORDER BY name")
                        .into_boxed::<Sqlite>(),
                )
                .load::<DynamicRow<SqliteAnyValue>>(conn)
        },
    )
}

#[test]
fn dynamic_columns_are_called_through_the_query_dsl() {
    let conn = &mut establish_connection();
    conn.batch_execute(
        "CREATE TABLE grants (role TEXT NOT NULL, name TEXT NOT NULL);
         INSERT INTO grants VALUES ('admin', 'bob'), ('admin', 'alice'), ('guest', 'carol');",
    )
    .unwrap();
    register_grantees(conn).unwrap();

    let grantees = table("grantees");
    let mut select = DynamicSelectClause::new();
    select.add_field(grantees.column::<Any, _>("name"));
    let admins = grantees
        .filter(grantees.column::<Text, _>("role").eq("admin"))
        .select(select)
        .load::<DynamicRow<SqliteAnyValue>>(conn)
        .unwrap();

    assert_eq!(
        admins,
        [
            DynamicRow::from(vec![SqliteAnyValue::Text("alice".into())]),
            DynamicRow::from(vec![SqliteAnyValue::Text("bob".into())]),
        ]
    );
}

#[test]
fn every_storage_class_round_trips() {
    let conn = &mut establish_connection();
    let identity = table("identity");
    conn.register_table_valued_function(
        vec![identity.column::<Any, _>("value")],
        vec![identity.column::<Any, _>("argument")],
        SqliteFunctionBehavior::INNOCUOUS,
        |_conn, arguments: DynamicRow<SqliteAnyValue>| Ok([arguments]),
    )
    .unwrap();

    for value in every_storage_class() {
        let row = value
            .bind_to(
                diesel::sql_query("SELECT typeof(value), value FROM identity(?1)").into_boxed(),
            )
            .get_result::<DynamicRow<SqliteAnyValue>>(conn)
            .unwrap();
        let storage_class = match value {
            SqliteAnyValue::Null => "null",
            SqliteAnyValue::Integer(_) => "integer",
            SqliteAnyValue::Real(_) => "real",
            SqliteAnyValue::Text(_) => "text",
            SqliteAnyValue::Blob(_) => "blob",
        };
        assert_eq!(row[0], SqliteAnyValue::Text(storage_class.into()));
        assert_eq!(row[1], value);
    }
}

#[test]
fn a_function_without_parameters_returns_every_row() {
    let conn = &mut establish_connection();
    let constants = table("constants");
    conn.register_table_valued_function(
        vec![constants.column::<Any, _>("value")],
        (),
        SqliteFunctionBehavior::INNOCUOUS,
        |_conn, ()| {
            Ok(every_storage_class()
                .into_iter()
                .map(|value| DynamicRow::from(vec![value])))
        },
    )
    .unwrap();

    let values = diesel::sql_query("SELECT value FROM constants")
        .load::<DynamicRow<SqliteAnyValue>>(conn)
        .unwrap()
        .into_iter()
        .map(|row| row[0].clone())
        .collect::<Vec<_>>();
    assert_eq!(values, every_storage_class());
}

#[test]
fn a_row_of_the_wrong_width_fails_the_statement() {
    let conn = &mut establish_connection();
    let pairs = table("pairs");
    conn.register_table_valued_function(
        vec![
            pairs.column::<Any, _>("left"),
            pairs.column::<Any, _>("right"),
        ],
        (),
        SqliteFunctionBehavior::INNOCUOUS,
        |_conn, ()| Ok([DynamicRow::from(vec![SqliteAnyValue::Integer(1)])]),
    )
    .unwrap();

    let error = diesel::sql_query("SELECT * FROM pairs")
        .load::<DynamicRow<SqliteAnyValue>>(conn)
        .unwrap_err();
    assert!(
        error.to_string().contains("has 1 values for 2 columns"),
        "{}",
        error
    );
}

#[test]
fn columns_of_another_table_are_refused() {
    let conn = &mut establish_connection();
    let error = conn
        .register_table_valued_function(
            vec![table("first").column::<Any, _>("value")],
            vec![table("second").column::<Any, _>("argument")],
            SqliteFunctionBehavior::INNOCUOUS,
            |_conn, arguments: DynamicRow<SqliteAnyValue>| Ok([arguments]),
        )
        .unwrap_err();
    match error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UnableToSendCommand,
            information,
        ) => assert_eq!(
            information.message(),
            "the column `argument` belongs to `second` and not to the table-valued function `first`"
        ),
        other => panic!("unexpected error {:?}", other),
    }
}

#[test]
fn a_function_with_a_schema_is_refused() {
    let conn = &mut establish_connection();
    let qualified = diesel_dynamic_schema::schema("main").table("qualified");
    let error = conn
        .register_table_valued_function(
            vec![qualified.column::<Any, _>("value")],
            (),
            SqliteFunctionBehavior::INNOCUOUS,
            |_conn, ()| Ok(Vec::<DynamicRow<SqliteAnyValue>>::new()),
        )
        .unwrap_err();
    assert!(
        error.to_string().contains("cannot have a schema"),
        "{}",
        error
    );
}

#[test]
fn a_trigger_calls_the_function_while_its_statement_runs() {
    let conn = &mut establish_connection();
    conn.batch_execute(
        "CREATE TABLE grants (role TEXT NOT NULL, name TEXT NOT NULL);
         INSERT INTO grants VALUES ('admin', 'bob'), ('admin', 'alice');
         CREATE TABLE requests (role TEXT NOT NULL);
         CREATE TABLE notified (name TEXT NOT NULL);
         CREATE TRIGGER notify AFTER INSERT ON requests BEGIN
             INSERT INTO notified SELECT name FROM grantees(NEW.role);
         END;",
    )
    .unwrap();
    register_grantees(conn).unwrap();

    conn.batch_execute("INSERT INTO requests VALUES ('admin')")
        .unwrap();

    let notified = diesel::sql_query("SELECT name FROM notified ORDER BY name")
        .load::<DynamicRow<SqliteAnyValue>>(conn)
        .unwrap();
    assert_eq!(
        notified,
        [
            DynamicRow::from(vec![SqliteAnyValue::Text("alice".into())]),
            DynamicRow::from(vec![SqliteAnyValue::Text("bob".into())]),
        ]
    );
}
