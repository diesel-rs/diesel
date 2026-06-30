//! Verify that the SQLite update hook works with `diesel_dynamic_schema`
//! tables, where the table type does not implement `StaticQueryFragment`.
//!
//! Dynamic schema users have no typed `table!` marker, so they either pass the
//! dynamic table to `SqliteUpdateRouter::on_dynamic` (which routes by its
//! runtime name) or use an `on_any` route that filters on the string
//! `table_name`, plus a partial op mask for op-level filtering.

#![cfg(feature = "sqlite")]

use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::sqlite::{SqliteChangeOp, SqliteChangeOps, SqliteUpdateRouter};
use diesel_dynamic_schema::table;
use std::sync::{Arc, Mutex};

fn establish_connection() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

/// Verify that an `on_update` closure fires for insert, update, and delete
/// operations performed via `sql_query` (the typical DML path for dynamic
/// schema users), and that `event.table_name` matches the runtime table name
/// used by `diesel_dynamic_schema::table`.
#[test]
fn update_hook_works_with_dynamic_schema() {
    let conn = &mut establish_connection();

    diesel::sql_query("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(conn)
        .unwrap();

    let items = table("items");
    let name = items.column::<Text, _>("name");

    let events: Arc<Mutex<Vec<(SqliteChangeOp, String, i64)>>> = Arc::new(Mutex::new(Vec::new()));
    let events_hook = events.clone();

    conn.on_update(
        SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
            events_hook
                .lock()
                .unwrap()
                .push((ev.op, ev.table_name.to_owned(), ev.rowid));
        }),
    );

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

/// Verify that a router built with `on_any` and a partial ops mask correctly
/// filters events when used alongside dynamic schema tables, without any
/// typed table marker.
#[test]
fn update_hook_ops_filter_with_dynamic_schema() {
    let conn = &mut establish_connection();

    diesel::sql_query("CREATE TABLE widgets (id INTEGER PRIMARY KEY, label TEXT NOT NULL)")
        .execute(conn)
        .unwrap();

    let events: Arc<Mutex<Vec<(SqliteChangeOp, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let events_hook = events.clone();

    conn.on_update(SqliteUpdateRouter::new().on_any(
        SqliteChangeOps::INSERT | SqliteChangeOps::DELETE,
        move |ev| {
            events_hook
                .lock()
                .unwrap()
                .push((ev.op, ev.table_name.to_owned()));
        },
    ));

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

/// Verify that a `diesel_dynamic_schema` table can be passed directly to
/// `SqliteUpdateRouter::on_dynamic`, routing by its runtime name without a
/// typed `table!` marker. A change to a different table must not fire it.
#[test]
fn on_dynamic_routes_to_a_dynamic_schema_table() {
    let conn = &mut establish_connection();

    diesel::sql_query("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(conn)
        .unwrap();
    diesel::sql_query("CREATE TABLE other (id INTEGER PRIMARY KEY)")
        .execute(conn)
        .unwrap();

    let items = table("items");
    let events: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let events_hook = events.clone();

    conn.on_update(
        SqliteUpdateRouter::new().on_dynamic(items, SqliteChangeOps::ALL, move |ev| {
            events_hook.lock().unwrap().push(ev.rowid);
        }),
    );

    // A change to a different table must not fire the items-only route.
    diesel::sql_query("INSERT INTO other (id) VALUES (1)")
        .execute(conn)
        .unwrap();
    // A change to items must fire it.
    diesel::sql_query("INSERT INTO items (id, name) VALUES (2, 'Widget')")
        .execute(conn)
        .unwrap();

    assert_eq!(
        *events.lock().unwrap(),
        vec![2],
        "on_dynamic should route only changes on the given dynamic table"
    );
}
