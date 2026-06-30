//! Types for the SQLite data change notification hook.
//!
//! See [`SqliteConnection::on_update`](super::SqliteConnection::on_update) for usage.

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::HasDatabaseAndTableName;
use crate::query_builder::nodes::{Identifier, StaticQueryFragment};
use alloc::borrow::Cow;
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
    /// if change.is_from::<users::table>() { /* ... */ }
    /// # }
    /// ```
    ///
    /// Only the table name is compared, not the database, so a same-named
    /// table in an `ATTACH`-ed database also matches. Inspect
    /// [`db_name`](Self::db_name) if you need to tell them apart.
    pub fn is_from<T>(&self) -> bool
    where
        T: StaticQueryFragment<Component = Identifier<'static>>,
    {
        self.table_name == T::STATIC_COMPONENT.0
    }

    /// Returns `Some(rowid)` if this change is on table `T`, otherwise `None`.
    /// Shorthand for [`is_from`](Self::is_from) followed by reading
    /// [`rowid`](Self::rowid):
    ///
    /// ```rust
    /// # diesel::table! { users (id) { id -> Integer, name -> Text, } }
    /// # fn f(change: &diesel::sqlite::SqliteChangeEvent<'_>) {
    /// if let Some(rowid) = change.rowid_in::<users::table>() { let _ = rowid; }
    /// # }
    /// ```
    pub fn rowid_in<T>(&self) -> Option<i64>
    where
        T: StaticQueryFragment<Component = Identifier<'static>>,
    {
        if self.is_from::<T>() {
            Some(self.rowid)
        } else {
            None
        }
    }
}

struct Route {
    database: Option<Cow<'static, str>>, // None = any database
    table: Option<Cow<'static, str>>,    // None = any table
    ops: SqliteChangeOps,
    callback: Box<dyn FnMut(SqliteChangeEvent<'_>) + Send>,
}

impl Route {
    fn matches(&self, event: &SqliteChangeEvent<'_>) -> bool {
        self.ops.matches_op(event.op)
            && self
                .table
                .as_deref()
                .is_none_or(|name| name == event.table_name)
            && self
                .database
                .as_deref()
                .is_none_or(|db| db == event.db_name)
    }
}

/// A table whose database and name are known only at runtime, used to route
/// change events through [`SqliteUpdateRouter::on_dynamic`].
///
/// This is implemented by `diesel_dynamic_schema` tables. Static
/// [`table!`](macro@crate::table) types do not need it: pass them to
/// [`on`](SqliteUpdateRouter::on) directly.
pub trait DynamicChangeTable {
    /// The database (schema) the table lives in, or `None` to match a table of
    /// this name in any database. Compared against
    /// [`SqliteChangeEvent::db_name`].
    fn change_database(&self) -> Option<Cow<'static, str>>;

    /// The table name, compared against [`SqliteChangeEvent::table_name`].
    fn change_table(&self) -> Cow<'static, str>;
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
    /// `table` is a [`table!`](macro@crate::table)-generated table value (for
    /// example `users::table`), taken by value so the call reads naturally, the
    /// same way [`get_read_only_blob`](super::SqliteConnection::get_read_only_blob)
    /// takes a column. Its name and optional schema are read at build time via
    /// [`StaticQueryFragment`]. An unqualified table matches by table name in
    /// any database, so a same-named table in an `ATTACH`-ed database also
    /// matches. A schema-qualified `table!` type additionally matches the
    /// database name, so it fires only for that attached database.
    pub fn on<T, F>(mut self, table: T, ops: SqliteChangeOps, callback: F) -> Self
    where
        T: StaticQueryFragment,
        T::Component: HasDatabaseAndTableName,
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        // The table value is taken only for ergonomic call syntax. Its name and
        // optional schema come from the type's static component.
        let _ = table;
        self.routes.push(Route {
            database: T::STATIC_COMPONENT.database_name().map(Cow::Borrowed),
            table: Some(Cow::Borrowed(T::STATIC_COMPONENT.table_name())),
            ops,
            callback: Box::new(callback),
        });
        self
    }

    /// Routes changes on a runtime-named `table` matching `ops` to `callback`.
    ///
    /// This is the dynamic counterpart to [`on`](Self::on). It accepts any
    /// [`DynamicChangeTable`], such as a `diesel_dynamic_schema` table whose
    /// name and schema are known only at runtime. As with [`on`](Self::on), a
    /// `None` schema matches the table name in any database and a `Some` schema
    /// matches only that attached database.
    pub fn on_dynamic<T, F>(mut self, table: T, ops: SqliteChangeOps, callback: F) -> Self
    where
        T: DynamicChangeTable,
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        self.routes.push(Route {
            database: table.change_database(),
            table: Some(table.change_table()),
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
            database: None,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
