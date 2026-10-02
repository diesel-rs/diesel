//! Types for the SQLite data change notification hook.
//!
//! See [`SqliteConnection::on_update`](super::SqliteConnection::on_update) for usage.

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;

use crate::query_source::NamedTable;
use alloc::boxed::Box;
use alloc::vec::Vec;

bitflags::bitflags! {
    /// A bitmask of SQLite change operations used for filtering which events
    /// a hook should receive.
    ///
    /// Combine masks with `|` (bitwise OR):
    ///
    /// ```rust
    /// # use diesel::sqlite::SqliteChangeOps;
    /// let insert_or_delete = SqliteChangeOps::INSERT | SqliteChangeOps::DELETE;
    /// assert!(insert_or_delete.contains(SqliteChangeOps::INSERT));
    /// assert!(!insert_or_delete.contains(SqliteChangeOps::UPDATE));
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SqliteChangeOps: u8 {
        /// Match INSERT operations.
        const INSERT = 1;
        /// Match UPDATE operations.
        const UPDATE = 2;
        /// Match DELETE operations.
        const DELETE = 4;
        /// Match unknown or future operation codes.
        const UNKNOWN = 8;
        /// Match all row-change operations (INSERT, UPDATE, DELETE, and UNKNOWN).
        ///
        /// `UNKNOWN` is included deliberately: if a future SQLite version emits
        /// an operation code diesel does not recognize, a hook registered with
        /// `ALL` still fires (with [`SqliteChangeOp::Unknown`] carrying the raw
        /// code) rather than silently dropping the change.
        const ALL =
            Self::INSERT.bits() | Self::UPDATE.bits() | Self::DELETE.bits() | Self::UNKNOWN.bits();
    }
}

impl SqliteChangeOps {
    /// Checks whether this mask includes the given [`SqliteChangeOp`].
    pub(crate) fn matches_op(self, op: SqliteChangeOp) -> bool {
        self.contains(op.to_ops())
    }
}

/// Identifies which kind of row change occurred.
///
/// Returned as part of [`SqliteChangeEvent`] to the callback registered via
/// [`on_update`](super::SqliteConnection::on_update).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqliteChangeOp {
    /// A row was inserted.
    Insert,
    /// A row was updated.
    Update,
    /// A row was deleted.
    Delete,
    /// An operation code this version of diesel does not recognize, for example
    /// one added by a future SQLite version. The inner value is the raw FFI code.
    Unknown(i32),
}

impl SqliteChangeOp {
    /// Converts a raw FFI operation code to the corresponding enum variant.
    pub(crate) fn from_ffi(code: i32) -> Self {
        #[allow(non_upper_case_globals)]
        match code {
            ffi::SQLITE_INSERT => SqliteChangeOp::Insert,
            ffi::SQLITE_UPDATE => SqliteChangeOp::Update,
            ffi::SQLITE_DELETE => SqliteChangeOp::Delete,
            other => SqliteChangeOp::Unknown(other),
        }
    }

    /// Converts this single operation to the corresponding [`SqliteChangeOps`]
    /// bitmask.
    pub(crate) fn to_ops(self) -> SqliteChangeOps {
        match self {
            SqliteChangeOp::Insert => SqliteChangeOps::INSERT,
            SqliteChangeOp::Update => SqliteChangeOps::UPDATE,
            SqliteChangeOp::Delete => SqliteChangeOps::DELETE,
            SqliteChangeOp::Unknown(_) => SqliteChangeOps::UNKNOWN,
        }
    }
}

/// Describes a single row change event from SQLite.
///
/// See: <https://www.sqlite.org/c3ref/update_hook.html>
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct SqliteChangeEvent<'a> {
    /// The operation that triggered this event.
    pub op: SqliteChangeOp,
    /// The name of the database the change occurred in: `"main"` for the
    /// primary database, `"temp"` for temporary tables, or the alias from an
    /// `ATTACH DATABASE` statement.
    pub db_name: &'a str,
    /// The name of the table that was modified.
    pub table_name: &'a str,
    /// SQLite's internal 64-bit [rowid](https://www.sqlite.org/rowidtable.html)
    /// of the affected row. For an `INTEGER PRIMARY KEY` table this equals the
    /// primary key, otherwise it is a separate hidden value.
    pub rowid: i64,
}

impl SqliteChangeEvent<'_> {
    /// Returns `true` if this change is on the [`table!`](macro@crate::table)
    /// table `T`, comparing [`table_name`](Self::table_name) against `T`'s
    /// name. Lets a callback match a typed table marker instead of a string
    /// literal:
    ///
    /// ```rust
    /// # diesel::table! { users (id) { id -> Integer, name -> Text, } }
    /// # fn f(change: &diesel::sqlite::SqliteChangeEvent<'_>) {
    /// if change.is_from(users::table) { /* ... */ }
    /// # }
    /// ```
    ///
    /// Only the table name is compared, not the database, so a same-named
    /// table in an `ATTACH`-ed database also matches. Inspect
    /// [`db_name`](Self::db_name) if you need to tell them apart.
    pub fn is_from(&self, table: impl NamedTable) -> bool {
        self.table_name == table.table() && table.schema().is_none_or(|db| self.db_name == db)
    }

    /// Returns `Some(rowid)` if this change is on table `T`, otherwise `None`.
    /// Shorthand for [`is_from`](Self::is_from) followed by reading
    /// [`rowid`](Self::rowid):
    ///
    /// ```rust
    /// # diesel::table! { users (id) { id -> Integer, name -> Text, } }
    /// # fn f(change: &diesel::sqlite::SqliteChangeEvent<'_>) {
    /// if let Some(rowid) = change.rowid_in(users::table) { let _ = rowid; }
    /// # }
    /// ```
    pub fn rowid_in(&self, table: impl NamedTable) -> Option<i64> {
        if self.is_from(table) {
            Some(self.rowid)
        } else {
            None
        }
    }
}

// A helper trait to use `NamedTable` with trait objects
// without requiring all the super type restrictions
trait DynNamedTable {
    fn schema(&self) -> Option<&str>;
    fn table(&self) -> &str;
}

impl<T> DynNamedTable for T
where
    T: NamedTable,
{
    fn schema(&self) -> Option<&str> {
        NamedTable::schema(self)
    }

    fn table(&self) -> &str {
        NamedTable::table(self)
    }
}

struct Route {
    table: Option<Box<dyn DynNamedTable + Send>>,
    ops: SqliteChangeOps,
    callback: Box<dyn FnMut(SqliteChangeEvent<'_>) + Send>,
}

impl Route {
    fn matches(&self, event: &SqliteChangeEvent<'_>) -> bool {
        self.ops.matches_op(event.op)
            && self.table.as_deref().is_none_or(|name| {
                name.table() == event.table_name
                    && name.schema().is_none_or(|db| db == event.db_name)
            })
    }
}

/// Routes SQLite row-change events to per-table callbacks, selected by typed
/// [`table!`](macro@crate::table) markers.
///
/// SQLite allows only one update hook per connection. This router is a single
/// value the caller composes and installs into that one slot with
/// [`on_update`](super::SqliteConnection::on_update), so the dispatch table is
/// explicit rather than hidden connection state. Build it with [`on`](Self::on)
/// and [`on_any`](Self::on_any), then install it:
///
/// ```rust
/// use diesel::prelude::*;
/// use diesel::sqlite::{SqliteConnection, SqliteChangeOps, SqliteUpdateRouter};
/// use std::sync::{Arc, Mutex};
///
/// diesel::table! { users (id) { id -> Integer, name -> Text, } }
///
/// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
/// # use diesel::connection::SimpleConnection;
/// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
/// let inserted = Arc::new(Mutex::new(Vec::new()));
/// let captured = inserted.clone();
///
/// conn.on_update(
///     SqliteUpdateRouter::new()
///         .on(users::table, SqliteChangeOps::INSERT, move |change| {
///             captured.lock().unwrap().push(change.rowid);
///         }),
/// );
///
/// diesel::insert_into(users::table)
///     .values(users::name.eq("Alice"))
///     .execute(conn)
///     .unwrap();
///
/// assert_eq!(*inserted.lock().unwrap(), vec![1]);
/// ```
///
/// Every matching route fires for a given event, so overlapping routes (for
/// example an [`on_any`](Self::on_any) audit log alongside table-specific
/// handlers) all run. There are no per-route handles: to change the routes,
/// install a different router or call
/// [`remove_update_hook`](super::SqliteConnection::remove_update_hook).
#[allow(missing_debug_implementations)]
pub struct SqliteUpdateRouter {
    routes: Vec<Route>,
}

impl SqliteUpdateRouter {
    /// Creates an empty router that matches nothing until routes are added.
    pub fn new() -> Self {
        SqliteUpdateRouter { routes: Vec::new() }
    }

    /// Routes changes on `table` matching `ops` to `callback`.
    ///
    /// `table` is a table type, either generated by [`table!`](macro@crate::table)
    /// or a diesel-dynamic-schema table. The schema name is treated as database name
    /// An unqualified table matches by table name in
    /// any database, so a same-named table in an `ATTACH`-ed database also
    /// matches. A schema-qualified `table!` type additionally matches the
    /// database name, so it fires only for that attached database.
    pub fn on<T, F>(mut self, table: T, ops: SqliteChangeOps, callback: F) -> Self
    where
        T: NamedTable + Send + 'static,
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        // The table value is taken only for ergonomic call syntax. Its name and
        // optional schema come from the type's static component.
        let _ = table;
        self.routes.push(Route {
            table: Some(Box::new(table)),
            ops,
            callback: Box::new(callback),
        });
        self
    }

    /// Routes changes on any table matching `ops` to `callback`.
    pub fn on_any<F>(mut self, ops: SqliteChangeOps, callback: F) -> Self
    where
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        self.routes.push(Route {
            table: None,
            ops,
            callback: Box::new(callback),
        });
        self
    }

    /// Dispatches an event to every matching route, in build order.
    fn dispatch(&mut self, event: SqliteChangeEvent<'_>) {
        for route in &mut self.routes {
            if route.matches(&event) {
                (route.callback)(event);
            }
        }
    }

    /// Turns the router into the callback installed by
    /// [`on_update`](super::SqliteConnection::on_update).
    pub(crate) fn into_hook(mut self) -> impl FnMut(SqliteChangeEvent<'_>) + Send {
        move |event| self.dispatch(event)
    }
}

impl Default for SqliteUpdateRouter {
    fn default() -> Self {
        SqliteUpdateRouter::new()
    }
}

impl SqliteConnection {
    /// Installs a [`SqliteUpdateRouter`](crate::sqlite::SqliteUpdateRouter) as
    /// the update hook, invoked for every row change (insert, update, or delete)
    /// on a [rowid table](https://www.sqlite.org/rowidtable.html). Replaces any
    /// previously registered update hook, since SQLite allows only one per
    /// connection.
    ///
    /// Build the router with [`SqliteUpdateRouter::on`] for typed per-table
    /// routes and [`SqliteUpdateRouter::on_any`] for a table-agnostic route. A
    /// single catch-all is `on_any(SqliteChangeOps::ALL, ...)`. Inside a
    /// callback, [`is_from`](crate::sqlite::SqliteChangeEvent::is_from) and
    /// [`rowid_in`](crate::sqlite::SqliteChangeEvent::rowid_in) match a `table!`
    /// marker without a string.
    ///
    /// Callbacks run synchronously as part of the `sqlite3_step()` call that
    /// performs the change, on the thread performing it, so they are never
    /// invoked concurrently. Per SQLite, a callback must not use the connection
    /// that triggered it (running any SQL, including a `SELECT`, counts as use)
    /// and is not reentrant. A panic in a callback aborts the process. To act on
    /// the changed row, capture its `rowid` and run the query after the
    /// statement completes.
    ///
    /// # Limitations
    ///
    /// These come from the underlying
    /// [`sqlite3_update_hook`](https://www.sqlite.org/c3ref/update_hook.html):
    ///
    /// - Only fires for [rowid tables](https://www.sqlite.org/rowidtable.html),
    ///   not `WITHOUT ROWID` tables.
    /// - Does not fire for changes to internal system tables, for `ON CONFLICT
    ///   REPLACE` deletions, or for the truncate optimization (`DELETE` with no
    ///   `WHERE` on a trigger-free table).
    ///
    /// See: [`sqlite3_update_hook`](https://www.sqlite.org/c3ref/update_hook.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteChangeOps, SqliteConnection, SqliteUpdateRouter};
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! { users (id) { id -> Integer, name -> Text, } }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let changes = Arc::new(Mutex::new(Vec::new()));
    /// let captured = changes.clone();
    ///
    /// conn.on_update(
    ///     SqliteUpdateRouter::new().on(
    ///         users::table,
    ///         SqliteChangeOps::INSERT,
    ///         move |change| captured.lock().unwrap().push(change.rowid),
    ///     ),
    /// );
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn)
    ///     .unwrap();
    ///
    /// assert_eq!(*changes.lock().unwrap(), vec![1]);
    /// ```
    pub fn on_update(&mut self, router: SqliteUpdateRouter) {
        self.raw_connection.set_update_hook(router.into_hook());
    }

    /// Removes the update hook. Subsequent row changes will not invoke any
    /// callback.
    ///
    /// See [`on_update`](Self::on_update) for usage.
    pub fn remove_update_hook(&mut self) {
        self.raw_connection.remove_update_hook();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::prelude::*;
    use crate::query_dsl::RunQueryDsl;

    /// Test-only helper: check mask vs raw FFI op code.
    impl SqliteChangeOps {
        fn matches(self, op_code: i32) -> bool {
            self.matches_op(SqliteChangeOp::from_ffi(op_code))
        }
    }

    #[test]
    fn insert_or_delete_matches_both_but_not_update() {
        let mask = SqliteChangeOps::INSERT | SqliteChangeOps::DELETE;
        assert!(mask.matches(ffi::SQLITE_INSERT));
        assert!(mask.matches(ffi::SQLITE_DELETE));
        assert!(!mask.matches(ffi::SQLITE_UPDATE));
    }

    #[test]
    fn all_matches_all_three() {
        assert!(SqliteChangeOps::ALL.matches(ffi::SQLITE_INSERT));
        assert!(SqliteChangeOps::ALL.matches(ffi::SQLITE_UPDATE));
        assert!(SqliteChangeOps::ALL.matches(ffi::SQLITE_DELETE));
    }

    #[test]
    fn combining_identical_masks_is_idempotent() {
        assert_eq!(
            SqliteChangeOps::INSERT | SqliteChangeOps::INSERT,
            SqliteChangeOps::INSERT,
        );
    }

    #[test]
    fn contains_single() {
        assert!(SqliteChangeOps::INSERT.contains(SqliteChangeOps::INSERT));
    }

    #[test]
    fn all_contains_insert_or_delete() {
        assert!(SqliteChangeOps::ALL.contains(SqliteChangeOps::INSERT | SqliteChangeOps::DELETE));
    }

    #[test]
    fn insert_does_not_contain_all() {
        assert!(!SqliteChangeOps::INSERT.contains(SqliteChangeOps::ALL));
    }

    #[test]
    fn from_ffi_insert() {
        assert_eq!(
            SqliteChangeOp::from_ffi(ffi::SQLITE_INSERT),
            SqliteChangeOp::Insert
        );
    }

    #[test]
    fn from_ffi_update() {
        assert_eq!(
            SqliteChangeOp::from_ffi(ffi::SQLITE_UPDATE),
            SqliteChangeOp::Update
        );
    }

    #[test]
    fn from_ffi_delete() {
        assert_eq!(
            SqliteChangeOp::from_ffi(ffi::SQLITE_DELETE),
            SqliteChangeOp::Delete
        );
    }

    #[test]
    fn to_ops_roundtrip() {
        assert_eq!(SqliteChangeOp::Insert.to_ops(), SqliteChangeOps::INSERT);
        assert_eq!(SqliteChangeOp::Update.to_ops(), SqliteChangeOps::UPDATE);
        assert_eq!(SqliteChangeOp::Delete.to_ops(), SqliteChangeOps::DELETE);
        assert_eq!(
            SqliteChangeOp::Unknown(999).to_ops(),
            SqliteChangeOps::UNKNOWN
        );
    }

    #[test]
    fn from_ffi_unknown_code() {
        assert_eq!(SqliteChangeOp::from_ffi(999), SqliteChangeOp::Unknown(999));
    }

    #[test]
    fn sqlite_change_event_is_copy() {
        let event = SqliteChangeEvent {
            op: SqliteChangeOp::Delete,
            db_name: "main",
            table_name: "posts",
            rowid: 7,
        };
        // Verify Copy by assigning to two bindings without a move error.
        let a = event;
        let b = event;
        assert_eq!(a.rowid, b.rowid);
    }

    #[test]
    fn debug_formatting() {
        // The `bitflags!`-derived Debug names single flags, the exact
        // composite rendering is bitflags' concern, so only check the names
        // appear and that empty and composite masks format without panicking.
        assert!(format!("{:?}", SqliteChangeOps::INSERT).contains("INSERT"));
        assert!(format!("{:?}", SqliteChangeOps::DELETE).contains("DELETE"));
        let _ = format!("{:?}", SqliteChangeOps::empty());
        let _ = format!("{:?}", SqliteChangeOps::ALL);
    }

    #[test]
    fn bitand_works() {
        let mask = SqliteChangeOps::ALL & SqliteChangeOps::INSERT;
        assert_eq!(mask, SqliteChangeOps::INSERT);
    }

    // -----------------------------------------------------------------------
    // Router unit tests (typed `on(table, ...)` routing is covered in hooks.rs,
    // which has real `table!` markers and a live connection)
    // -----------------------------------------------------------------------

    fn make_event(
        op: SqliteChangeOp,
        table: &'static str,
        rowid: i64,
    ) -> SqliteChangeEvent<'static> {
        SqliteChangeEvent {
            op,
            db_name: "main",
            table_name: table,
            rowid,
        }
    }

    #[test]
    fn empty_router_dispatches_nothing() {
        let mut router = SqliteUpdateRouter::new();
        // Must not panic when there are no routes.
        router.dispatch(make_event(SqliteChangeOp::Insert, "users", 1));
    }

    #[test]
    fn on_any_dispatches_for_every_table() {
        let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let f2 = fired.clone();
        let mut router = SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |e| {
            f2.lock().unwrap().push((e.op, e.rowid));
        });

        router.dispatch(make_event(SqliteChangeOp::Insert, "users", 1));
        router.dispatch(make_event(SqliteChangeOp::Delete, "posts", 2));

        assert_eq!(
            *fired.lock().unwrap(),
            vec![(SqliteChangeOp::Insert, 1), (SqliteChangeOp::Delete, 2)],
        );
    }

    #[test]
    fn router_filters_by_op_mask() {
        let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let f2 = fired.clone();
        let mut router = SqliteUpdateRouter::new().on_any(SqliteChangeOps::INSERT, move |e| {
            f2.lock().unwrap().push(e.rowid);
        });

        router.dispatch(make_event(SqliteChangeOp::Insert, "users", 1));
        router.dispatch(make_event(SqliteChangeOp::Update, "users", 2)); // filtered out
        router.dispatch(make_event(SqliteChangeOp::Delete, "users", 3)); // filtered out

        assert_eq!(*fired.lock().unwrap(), vec![1]);
    }

    #[test]
    fn every_matching_route_fires() {
        let count = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let c1 = count.clone();
        let c2 = count.clone();
        let mut router = SqliteUpdateRouter::new()
            .on_any(SqliteChangeOps::ALL, move |_| {
                *c1.lock().unwrap() += 1;
            })
            .on_any(SqliteChangeOps::INSERT, move |_| {
                *c2.lock().unwrap() += 1;
            });

        // Insert matches both routes, delete matches only the first.
        router.dispatch(make_event(SqliteChangeOp::Insert, "users", 1));
        router.dispatch(make_event(SqliteChangeOp::Delete, "users", 2));

        assert_eq!(*count.lock().unwrap(), 3);
    }

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    // ===================================================================
    // Change-hook integration tests
    // ===================================================================

    table! {
        hook_users {
            id -> Integer,
            name -> Text,
        }
    }

    table! {
        hook_posts {
            id -> Integer,
            title -> Text,
        }
    }

    fn setup_hook_tables(conn: &mut SqliteConnection) {
        crate::sql_query(
            "CREATE TABLE hook_users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();
        crate::sql_query(
            "CREATE TABLE hook_posts (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();
    }

    // A schema-qualified `table!` marker routes only to its attached database,
    // even when a same-named table exists in `main`.
    #[diesel_test_helper::test]
    fn router_on_matches_schema_qualified_table() {
        use std::sync::{Arc, Mutex};

        table! {
            attached.shared_items (id) {
                id -> Integer,
            }
        }

        let conn = &mut connection();
        crate::sql_query("ATTACH DATABASE ':memory:' AS attached")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE shared_items (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE attached.shared_items (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();
        conn.on_update(SqliteUpdateRouter::new().on(
            shared_items::table,
            SqliteChangeOps::ALL,
            move |ev| {
                f2.lock().unwrap().push((ev.db_name.to_owned(), ev.rowid));
            },
        ));

        // A change in `main.shared_items` must not fire the attached-only route.
        crate::sql_query("INSERT INTO main.shared_items (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        // A change in `attached.shared_items` must fire it.
        crate::sql_query("INSERT INTO attached.shared_items (id) VALUES (2)")
            .execute(conn)
            .unwrap();

        assert_eq!(
            *fired.lock().unwrap(),
            vec![("attached".to_owned(), 2)],
            "a schema-qualified route matches only its attached database"
        );
    }

    #[diesel_test_helper::test]
    fn router_on_dispatches_to_typed_table() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let fired2 = fired.clone();

        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::INSERT,
            move |change| {
                fired2.lock().unwrap().push((change.op, change.rowid));
            },
        ));

        // INSERT a row: the route fires immediately during sqlite3_step().
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(conn)
            .unwrap();

        let events = fired.lock().unwrap().clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, SqliteChangeOp::Insert);
        assert_eq!(events[0].1, 1); // rowid
    }

    #[diesel_test_helper::test]
    fn on_delete_fires_only_for_delete() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let fired2 = fired.clone();

        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::DELETE,
            move |change| {
                fired2.lock().unwrap().push(change.op);
            },
        ));

        // INSERT + UPDATE + DELETE
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'Bob' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();

        let events = fired.lock().unwrap().clone();
        // Only the DELETE should have matched.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], SqliteChangeOp::Delete);
    }

    #[diesel_test_helper::test]
    fn every_matching_route_fires_in_order() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let order = Arc::new(Mutex::new(Vec::new()));
        let o1 = order.clone();
        let o2 = order.clone();

        conn.on_update(
            SqliteUpdateRouter::new()
                .on(hook_users::table, SqliteChangeOps::INSERT, move |_| {
                    o1.lock().unwrap().push(1);
                })
                .on(hook_users::table, SqliteChangeOps::INSERT, move |_| {
                    o2.lock().unwrap().push(2);
                }),
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('X')")
            .execute(conn)
            .unwrap();

        assert_eq!(*order.lock().unwrap(), vec![1, 2]);
    }

    #[diesel_test_helper::test]
    fn remove_update_stops_dispatch() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(0u32));
        let f2 = fired.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |_| {
                *f2.lock().unwrap() += 1;
            }),
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        assert_eq!(*fired.lock().unwrap(), 1);

        // Remove the hook.
        conn.remove_update_hook();

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('B')")
            .execute(conn)
            .unwrap();
        // Should still be 1, hook was removed.
        assert_eq!(*fired.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn events_fire_immediately_during_statement() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        // Insert a row without a hook first.
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Z')")
            .execute(conn)
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::UPDATE,
            move |event| {
                f2.lock().unwrap().push(event.rowid);
            },
        ));

        // UPDATE triggers the C hook immediately during sqlite3_step().
        crate::sql_query("UPDATE hook_users SET name = 'W' WHERE id = 1")
            .execute(conn)
            .unwrap();

        assert_eq!(*fired.lock().unwrap(), vec![1i64]);
    }

    #[diesel_test_helper::test]
    fn on_update_fires_for_update_only() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::UPDATE,
            move |event| {
                assert_eq!(event.op, SqliteChangeOp::Update);
                *c2.lock().unwrap() += 1;
            },
        ));

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'B' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();

        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn on_update_receives_every_change() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let events = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                e2.lock().unwrap().push((ev.op, ev.table_name.to_owned()));
            }),
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('P')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'B' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_posts WHERE id = 1")
            .execute(conn)
            .unwrap();

        let evts = events.lock().unwrap().clone();
        assert_eq!(evts.len(), 4);
        assert_eq!(evts[0], (SqliteChangeOp::Insert, "hook_users".to_owned()));
        assert_eq!(evts[1], (SqliteChangeOp::Insert, "hook_posts".to_owned()));
        assert_eq!(evts[2], (SqliteChangeOp::Update, "hook_users".to_owned()));
        assert_eq!(evts[3], (SqliteChangeOp::Delete, "hook_posts".to_owned()));
    }

    #[diesel_test_helper::test]
    fn on_update_filters_by_op_mask() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_update(SqliteUpdateRouter::new().on_any(
            SqliteChangeOps::INSERT | SqliteChangeOps::DELETE,
            move |_| {
                *c2.lock().unwrap() += 1;
            },
        ));

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'B' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();

        // INSERT + DELETE = 2, not UPDATE
        assert_eq!(*count.lock().unwrap(), 2);
    }

    #[diesel_test_helper::test]
    fn router_dispatches_to_multiple_tables() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let user_count = Arc::new(Mutex::new(0u32));
        let post_count = Arc::new(Mutex::new(0u32));
        let uc = user_count.clone();
        let pc = post_count.clone();

        conn.on_update(
            SqliteUpdateRouter::new()
                .on(hook_users::table, SqliteChangeOps::ALL, move |_| {
                    *uc.lock().unwrap() += 1;
                })
                .on(hook_posts::table, SqliteChangeOps::ALL, move |_| {
                    *pc.lock().unwrap() += 1;
                }),
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('X')")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('Y')")
            .execute(conn)
            .unwrap();

        assert_eq!(*user_count.lock().unwrap(), 1);
        assert_eq!(*post_count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn on_any_audit_plus_specific_route() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let audit_count = Arc::new(Mutex::new(0u32));
        let user_insert_count = Arc::new(Mutex::new(0u32));
        let ac = audit_count.clone();
        let uic = user_insert_count.clone();

        conn.on_update(
            SqliteUpdateRouter::new()
                .on_any(SqliteChangeOps::ALL, move |_| {
                    *ac.lock().unwrap() += 1;
                })
                .on(hook_users::table, SqliteChangeOps::INSERT, move |_| {
                    *uic.lock().unwrap() += 1;
                }),
        );

        // Hits both the audit route and the user-insert route.
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('X')")
            .execute(conn)
            .unwrap();
        // Hits only the audit route.
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('Y')")
            .execute(conn)
            .unwrap();

        assert_eq!(*audit_count.lock().unwrap(), 2);
        assert_eq!(*user_insert_count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn rowid_in_filters_by_table() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let c2 = captured.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |change| {
                if let Some(rowid) = change.rowid_in(hook_users::table) {
                    c2.lock().unwrap().push(rowid);
                }
            }),
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('P')")
            .execute(conn)
            .unwrap();

        // Only the hook_users rowid was captured.
        assert_eq!(*captured.lock().unwrap(), vec![1i64]);
    }

    #[diesel_test_helper::test]
    fn is_from_matches_table_marker() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let c2 = captured.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |change| {
                c2.lock().unwrap().push((
                    change.is_from(hook_users::table),
                    change.is_from(hook_posts::table),
                ));
            }),
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();

        assert_eq!(*captured.lock().unwrap(), vec![(true, false)]);
    }

    #[diesel_test_helper::test]
    fn hooks_fire_across_transactions() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        // Register hook BEFORE the transaction.
        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::INSERT,
            move |event| {
                f2.lock().unwrap().push(event.rowid);
            },
        ));

        conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO hook_users (name) VALUES ('TxUser')")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(fired.lock().unwrap().len(), 1);
    }

    // ===================================================================
    // Negative tests: cases where the update hook must NOT fire
    //
    // These are documented SQLite limitations of sqlite3_update_hook():
    // https://www.sqlite.org/c3ref/update_hook.html
    // ===================================================================

    /// The update hook is not invoked for WITHOUT ROWID tables.
    /// See: https://www.sqlite.org/c3ref/update_hook.html
    #[diesel_test_helper::test]
    fn update_hook_silent_for_without_rowid_tables() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE kv (key TEXT PRIMARY KEY, val TEXT NOT NULL) WITHOUT ROWID")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<SqliteChangeOp>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                if ev.table_name == "kv" {
                    e2.lock().unwrap().push(ev.op);
                }
            }),
        );

        crate::sql_query("INSERT INTO kv (key, val) VALUES ('a', '1')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE kv SET val = '2' WHERE key = 'a'")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM kv WHERE key = 'a'")
            .execute(conn)
            .unwrap();

        assert!(
            events.lock().unwrap().is_empty(),
            "update hook must not fire for WITHOUT ROWID tables"
        );
    }

    /// When a UNIQUE constraint conflict is resolved via ON CONFLICT REPLACE,
    /// the implicit deletion of the conflicting row does NOT fire the update
    /// hook. Only the INSERT for the new row fires.
    /// See: https://www.sqlite.org/c3ref/update_hook.html
    #[diesel_test_helper::test]
    fn update_hook_silent_for_on_conflict_replace_deletion() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE uq (id INTEGER PRIMARY KEY, val TEXT NOT NULL UNIQUE)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO uq (id, val) VALUES (1, 'original')")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<(SqliteChangeOp, i64)>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                if ev.table_name == "uq" {
                    e2.lock().unwrap().push((ev.op, ev.rowid));
                }
            }),
        );

        // INSERT OR REPLACE with a conflicting val: the old row (id=1) is
        // silently deleted by SQLite and the new row (id=2) is inserted.
        // The hook fires only for the INSERT of the new row.
        crate::sql_query("INSERT OR REPLACE INTO uq (id, val) VALUES (2, 'original')")
            .execute(conn)
            .unwrap();

        let recorded = events.lock().unwrap();
        assert_eq!(
            recorded.len(),
            1,
            "expected only 1 event (INSERT), got: {:?}",
            *recorded
        );
        assert_eq!(recorded[0].0, SqliteChangeOp::Insert);
        assert_eq!(recorded[0].1, 2, "new row should have rowid 2");
    }

    /// DELETE FROM without a WHERE clause triggers the truncate optimization,
    /// which bypasses the update hook entirely: no per-row DELETE events fire.
    /// See: https://www.sqlite.org/lang_delete.html#truncateopt
    #[diesel_test_helper::test]
    fn update_hook_silent_for_truncate_optimization() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        // The truncate optimization applies when:
        //   1. No WHERE clause
        //   2. No RETURNING clause
        //   3. No triggers on the table
        crate::sql_query("CREATE TABLE bulk (id INTEGER PRIMARY KEY, data TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO bulk (data) VALUES ('a'), ('b'), ('c')")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<SqliteChangeOp>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                if ev.table_name == "bulk" {
                    e2.lock().unwrap().push(ev.op);
                }
            }),
        );

        // DELETE without WHERE: truncate optimization kicks in.
        crate::sql_query("DELETE FROM bulk").execute(conn).unwrap();

        assert!(
            events.lock().unwrap().is_empty(),
            "truncate optimization should bypass the update hook"
        );
    }

    /// When a table has triggers, the truncate optimization is disabled, so
    /// DELETE without WHERE fires per-row DELETE events as normal.
    #[diesel_test_helper::test]
    fn update_hook_fires_for_delete_all_when_triggers_disable_truncate() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE triggered (id INTEGER PRIMARY KEY, data TEXT NOT NULL)")
            .execute(conn)
            .unwrap();
        // A no-op trigger is enough to disable the truncate optimization.
        crate::sql_query(
            "CREATE TRIGGER trg_triggered BEFORE DELETE ON triggered \
             BEGIN SELECT 1; END",
        )
        .execute(conn)
        .unwrap();

        crate::sql_query("INSERT INTO triggered (data) VALUES ('x'), ('y'), ('z')")
            .execute(conn)
            .unwrap();

        let deletes: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
        let d2 = deletes.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                if ev.table_name == "triggered" && ev.op == SqliteChangeOp::Delete {
                    d2.lock().unwrap().push(ev.rowid);
                }
            }),
        );

        // DELETE without WHERE, but triggers exist ⇒ no truncate optimization.
        crate::sql_query("DELETE FROM triggered")
            .execute(conn)
            .unwrap();

        assert_eq!(
            deletes.lock().unwrap().len(),
            3,
            "with triggers present, DELETE without WHERE fires per-row hooks"
        );
    }

    /// Modifications to internal system tables like sqlite_sequence
    /// (used by AUTOINCREMENT) do not trigger the update hook.
    /// See: https://www.sqlite.org/c3ref/update_hook.html
    #[diesel_test_helper::test]
    fn update_hook_silent_for_internal_sqlite_sequence() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        // AUTOINCREMENT causes SQLite to maintain sqlite_sequence.
        crate::sql_query(
            "CREATE TABLE seq_test (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();

        let tables: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let t2 = tables.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                t2.lock().unwrap().push(ev.table_name.to_owned());
            }),
        );

        crate::sql_query("INSERT INTO seq_test (name) VALUES ('row1')")
            .execute(conn)
            .unwrap();

        let recorded = tables.lock().unwrap();
        // Only the user table should appear, sqlite_sequence must be absent.
        assert!(
            recorded.iter().all(|t| t == "seq_test"),
            "expected only 'seq_test' events, got: {:?}",
            *recorded
        );
        assert!(
            !recorded.iter().any(|t| t == "sqlite_sequence"),
            "sqlite_sequence modifications must not trigger the update hook"
        );
    }

    /// INSERT OR REPLACE on the primary key itself: when a row with the same
    /// PK already exists, the old row is silently deleted and the new row is
    /// inserted. The hook reports only the INSERT, not the implicit DELETE.
    #[diesel_test_helper::test]
    fn update_hook_silent_for_replace_into_on_pk_conflict() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE rep (id INTEGER PRIMARY KEY, val TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO rep (id, val) VALUES (1, 'old')")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<(SqliteChangeOp, i64)>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_update(
            SqliteUpdateRouter::new().on_any(SqliteChangeOps::ALL, move |ev| {
                if ev.table_name == "rep" {
                    e2.lock().unwrap().push((ev.op, ev.rowid));
                }
            }),
        );

        // REPLACE INTO with a conflicting PK.
        crate::sql_query("REPLACE INTO rep (id, val) VALUES (1, 'new')")
            .execute(conn)
            .unwrap();

        let recorded = events.lock().unwrap();
        // Only one INSERT event, no DELETE for the old row.
        assert_eq!(
            recorded.len(),
            1,
            "expected 1 event for REPLACE INTO, got: {:?}",
            *recorded
        );
        assert_eq!(recorded[0].0, SqliteChangeOp::Insert);
        assert_eq!(recorded[0].1, 1);
    }

    /// Regression test for the dangling-pointer soundness bug: after the
    /// connection is moved, a write still fires the registered callback.
    ///
    /// `sqlite3_update_hook` is handed the address of the connection's update
    /// hook state at registration time. Because that state is boxed, moving the
    /// (freely movable, `Sized`) `SqliteConnection` does not relocate it, so the
    /// pointer SQLite holds stays valid. On a buggy inline implementation the
    /// move would relocate the state and the trampoline would later dereference
    /// freed memory. This documents that the feature works through the move that
    /// real code performs (returning a connection, storing it in a struct,
    /// handing it to a pool, etc.).
    #[diesel_test_helper::test]
    fn change_hook_fires_after_connection_move() {
        use std::sync::{Arc, Mutex};

        let count = Arc::new(Mutex::new(0u32));
        let count2 = count.clone();

        let mut conn = connection();
        setup_hook_tables(&mut conn);
        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::INSERT,
            move |_| {
                *count2.lock().unwrap() += 1;
            },
        ));

        // Move the connection onto the heap after the hook was registered.
        let mut boxed = Box::new(conn);

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(&mut *boxed)
            .unwrap();

        assert_eq!(
            *count.lock().unwrap(),
            1,
            "change hook did not fire after the connection was moved"
        );
    }

    #[diesel_test_helper::test]
    fn router_filters_table_and_op() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let fired2 = fired.clone();

        // Fire for INSERT and UPDATE on hook_users only, not DELETE.
        conn.on_update(SqliteUpdateRouter::new().on(
            hook_users::table,
            SqliteChangeOps::INSERT | SqliteChangeOps::UPDATE,
            move |event| fired2.lock().unwrap().push(event.op),
        ));

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'Bob' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();
        // A change on a different table must not fire this hook.
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('Hello')")
            .execute(conn)
            .unwrap();

        let events = fired.lock().unwrap().clone();
        assert_eq!(events, vec![SqliteChangeOp::Insert, SqliteChangeOp::Update]);
    }
}
