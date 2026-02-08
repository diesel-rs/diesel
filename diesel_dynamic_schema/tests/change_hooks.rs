//! Verify that `on_change` hooks work with `diesel_dynamic_schema`
//! tables, where the table type does not implement `StaticQueryFragment`.
//!
//! `on_change` is the correct API for dynamic schema users because it
//! accepts no generic table type parameter and filters purely on string
//! comparison of the table name.

#![cfg(feature = "sqlite")]

use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::sqlite::{SqliteChangeEvent, SqliteChangeOp, SqliteChangeOps};
use diesel_dynamic_schema::table;
use std::sync::{Arc, Mutex};

fn establish_connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

/// Verify that the untyped `on_change` hook fires for insert, update,
/// and delete operations performed via `sql_query` (the typical DML path
/// for dynamic schema users), and that `event.table_name` matches the
/// runtime table name used by `diesel_dynamic_schema::table`.
#[test]
fn on_change_works_with_dynamic_schema() {
    let conn = &mut establish_connection();

    diesel::sql_query(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
    )
    .execute(conn)
    .unwrap();

    let items = table("items");
    let name = items.column::<Text, _>("name");

    let events: Arc<Mutex<Vec<(SqliteChangeOp, String, i64)>>> =
        Arc::new(Mutex::new(Vec::new()));
    let events_hook = events.clone();

    conn.on_change(SqliteChangeOps::ALL, move |ev: SqliteChangeEvent<'_>| {
        events_hook
            .lock()
            .unwrap()
            .push((ev.op, ev.table_name.to_owned(), ev.rowid));
    });

    diesel::sql_query("INSERT INTO items (id, name) VALUES (1, 'Widget')")
        .execute(conn)
        .unwrap();
    diesel::sql_query("UPDATE items SET name = 'Gizmo' WHERE id = 1")
        .execute(conn)
        .unwrap();
    diesel::sql_query("DELETE FROM items WHERE id = 1")
        .execute(conn)
        .unwrap();

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 3);

    assert_eq!(recorded[0], (SqliteChangeOp::Insert, "items".into(), 1));
    assert_eq!(recorded[1], (SqliteChangeOp::Update, "items".into(), 1));
    assert_eq!(recorded[2], (SqliteChangeOp::Delete, "items".into(), 1));
    drop(recorded);

    // Verify that queries via the dynamic schema table still work
    // alongside an active hook.
    diesel::sql_query("INSERT INTO items (id, name) VALUES (2, 'Doohickey')")
        .execute(conn)
        .unwrap();

    let names = items.select(name).load::<String>(conn).unwrap();
    assert_eq!(names, vec!["Doohickey"]);

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 4);
    assert_eq!(recorded[3].0, SqliteChangeOp::Insert);
}

/// Verify that `on_change` with a partial ops mask correctly filters
/// events when used alongside dynamic schema tables.
#[test]
fn on_change_ops_filter_with_dynamic_schema() {
    let conn = &mut establish_connection();

    diesel::sql_query(
        "CREATE TABLE widgets (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
    )
    .execute(conn)
    .unwrap();

    let events: Arc<Mutex<Vec<(SqliteChangeOp, String)>>> =
        Arc::new(Mutex::new(Vec::new()));
    let events_hook = events.clone();

    conn.on_change(
        SqliteChangeOps::INSERT | SqliteChangeOps::DELETE,
        move |ev: SqliteChangeEvent<'_>| {
            events_hook
                .lock()
                .unwrap()
                .push((ev.op, ev.table_name.to_owned()));
        },
    );

    diesel::sql_query("INSERT INTO widgets (id, label) VALUES (1, 'Alpha')")
        .execute(conn)
        .unwrap();
    diesel::sql_query("UPDATE widgets SET label = 'Beta' WHERE id = 1")
        .execute(conn)
        .unwrap();
    diesel::sql_query("DELETE FROM widgets WHERE id = 1")
        .execute(conn)
        .unwrap();

    let recorded = events.lock().unwrap();
    assert_eq!(
        recorded.len(),
        2,
        "expected INSERT + DELETE only, got {:?}",
        *recorded
    );
    assert_eq!(recorded[0], (SqliteChangeOp::Insert, "widgets".into()));
    assert_eq!(recorded[1], (SqliteChangeOp::Delete, "widgets".into()));
}
