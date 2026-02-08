//! Types for SQLite data change notification hooks.
//!
//! See [`SqliteConnection::on_change`] and related methods for usage.

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr};

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
///
/// The inner representation is private to prevent construction of invalid masks.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SqliteChangeOps(u8);

impl SqliteChangeOps {
    /// Match INSERT operations.
    pub const INSERT: SqliteChangeOps = SqliteChangeOps(1);
    /// Match UPDATE operations.
    pub const UPDATE: SqliteChangeOps = SqliteChangeOps(2);
    /// Match DELETE operations.
    pub const DELETE: SqliteChangeOps = SqliteChangeOps(4);
    /// Match unknown/future operation codes.
    pub const UNKNOWN: SqliteChangeOps = SqliteChangeOps(8);
    /// Match all row-change operations (INSERT, UPDATE, DELETE, and UNKNOWN).
    pub const ALL: SqliteChangeOps = SqliteChangeOps(15);

    /// Returns `true` if `self` is a superset of `other` — i.e. every
    /// operation bit set in `other` is also set in `self`.
    pub fn contains(self, other: SqliteChangeOps) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Checks whether this mask includes the given [`SqliteChangeOp`].
    pub(crate) fn matches_op(self, op: SqliteChangeOp) -> bool {
        self.contains(op.to_ops())
    }
}

impl BitOr for SqliteChangeOps {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        SqliteChangeOps(self.0 | rhs.0)
    }
}

impl BitAnd for SqliteChangeOps {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        SqliteChangeOps(self.0 & rhs.0)
    }
}

impl core::fmt::Debug for SqliteChangeOps {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut parts = Vec::new();
        if self.contains(Self::INSERT) {
            parts.push("INSERT");
        }
        if self.contains(Self::UPDATE) {
            parts.push("UPDATE");
        }
        if self.contains(Self::DELETE) {
            parts.push("DELETE");
        }
        if self.contains(Self::UNKNOWN) {
            parts.push("UNKNOWN");
        }
        if parts.is_empty() {
            write!(f, "SqliteChangeOps(0)")
        } else {
            write!(f, "SqliteChangeOps({})", parts.join(" | "))
        }
    }
}

// ---------------------------------------------------------------------------
// SqliteChangeOp — runtime enum
// ---------------------------------------------------------------------------

/// Identifies which kind of row change occurred.
///
/// Returned as part of [`SqliteChangeEvent`] to callbacks registered via
/// [`SqliteConnection::on_insert`], [`SqliteConnection::on_change`], etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqliteChangeOp {
    /// A row was inserted.
    Insert,
    /// A row was updated.
    Update,
    /// A row was deleted.
    Delete,
    /// An operation code not recognised by this version of diesel.
    ///
    /// This can happen if a future SQLite version introduces new op codes.
    /// The inner value is the raw FFI code.
    Unknown(i32),
}

impl SqliteChangeOp {
    /// Converts a raw FFI operation code to the corresponding enum variant.
    ///
    /// Unknown codes are mapped to [`SqliteChangeOp::Unknown`].
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
    pub fn to_ops(self) -> SqliteChangeOps {
        match self {
            SqliteChangeOp::Insert => SqliteChangeOps::INSERT,
            SqliteChangeOp::Update => SqliteChangeOps::UPDATE,
            SqliteChangeOp::Delete => SqliteChangeOps::DELETE,
            SqliteChangeOp::Unknown(_) => SqliteChangeOps::UNKNOWN,
        }
    }
}

// ---------------------------------------------------------------------------
// SqliteChangeEvent — borrowed event passed to callbacks
// ---------------------------------------------------------------------------

/// Describes a single row change event from SQLite.
///
/// The `db_name` field identifies which attached database the change occurred
/// in — `"main"` for the primary database, `"temp"` for temporary tables,
/// or the alias from an `ATTACH DATABASE` statement.
///
/// The `table_name` field is the name of the table that was modified.
///
/// The `rowid` field is SQLite's internal 64-bit row identifier. For tables
/// with an `INTEGER PRIMARY KEY`, this is the same value as the primary key
/// column. For tables with other primary key types, the rowid is a separate
/// hidden value managed by SQLite internally.
///
/// See: <https://www.sqlite.org/c3ref/update_hook.html>
/// See: <https://www.sqlite.org/rowidtable.html>
#[derive(Debug, Clone, Copy)]
pub struct SqliteChangeEvent<'a> {
    /// The operation that triggered this event.
    pub op: SqliteChangeOp,
    /// The name of the database (e.g. `"main"`, `"temp"`, or an `ATTACH` alias).
    pub db_name: &'a str,
    /// The name of the table that was modified.
    pub table_name: &'a str,
    /// The rowid of the affected row.
    pub rowid: i64,
}

/// An opaque handle to a registered change hook, used to remove it later
/// via [`SqliteConnection::remove_change_hook`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeHookId(pub(crate) usize);

pub(crate) struct HookEntry {
    id: usize,
    table_name: Option<&'static str>, // None = catch-all (on_change)
    ops: SqliteChangeOps,
    callback: Box<dyn FnMut(SqliteChangeEvent<'_>) + Send>,
}

impl HookEntry {
    pub(crate) fn matches(&self, event: &SqliteChangeEvent<'_>) -> bool {
        self.ops.matches_op(event.op) && self.table_name.is_none_or(|name| name == event.table_name)
    }
}

pub(crate) struct ChangeHookDispatcher {
    next_id: usize,
    pub(crate) entries: Vec<HookEntry>,
}

impl ChangeHookDispatcher {
    pub(crate) fn new() -> Self {
        ChangeHookDispatcher {
            next_id: 0,
            entries: Vec::new(),
        }
    }

    pub(crate) fn add(
        &mut self,
        table_name: Option<&'static str>,
        ops: SqliteChangeOps,
        callback: Box<dyn FnMut(SqliteChangeEvent<'_>) + Send>,
    ) -> ChangeHookId {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(HookEntry {
            id,
            table_name,
            ops,
            callback,
        });
        ChangeHookId(id)
    }

    /// Dispatches a change event to all matching hooks.
    pub(crate) fn dispatch(&mut self, event: SqliteChangeEvent<'_>) {
        for entry in &mut self.entries {
            if entry.matches(&event) {
                (entry.callback)(event);
            }
        }
    }

    pub(crate) fn remove(&mut self, hook_id: ChangeHookId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != hook_id.0);
        self.entries.len() < before
    }

    pub(crate) fn clear_for_table(&mut self, table_name: &str) {
        self.entries.retain(|e| e.table_name != Some(table_name));
    }

    pub(crate) fn clear_all(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Maps an FFI operation code to our internal bitmask bit.
#[cfg(test)]
fn ffi_op_to_bit(op_code: i32) -> u8 {
    #[allow(non_upper_case_globals)]
    match op_code {
        ffi::SQLITE_INSERT => SqliteChangeOps::INSERT.0,
        ffi::SQLITE_UPDATE => SqliteChangeOps::UPDATE.0,
        ffi::SQLITE_DELETE => SqliteChangeOps::DELETE.0,
        _ => SqliteChangeOps::UNKNOWN.0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only helper: check mask vs raw FFI op code.
    impl SqliteChangeOps {
        fn matches(self, op_code: i32) -> bool {
            let bit = ffi_op_to_bit(op_code);
            (self.0 & bit) != 0
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
        assert_eq!(
            format!("{:?}", SqliteChangeOps::INSERT),
            "SqliteChangeOps(INSERT)",
        );
        assert_eq!(
            format!("{:?}", SqliteChangeOps::ALL),
            "SqliteChangeOps(INSERT | UPDATE | DELETE | UNKNOWN)",
        );
        assert_eq!(format!("{:?}", SqliteChangeOps(0)), "SqliteChangeOps(0)",);
    }

    #[test]
    fn bitand_works() {
        let mask = SqliteChangeOps::ALL & SqliteChangeOps::INSERT;
        assert_eq!(mask, SqliteChangeOps::INSERT);
    }

    // -----------------------------------------------------------------------
    // Dispatcher unit tests
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
    fn dispatcher_add_matches_remove() {
        let mut d = ChangeHookDispatcher::new();
        assert!(d.is_empty());

        let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let fired2 = fired.clone();

        let id = d.add(
            Some("users"),
            SqliteChangeOps::INSERT,
            Box::new(move |e| {
                fired2.lock().unwrap().push(e.rowid);
            }),
        );
        assert!(!d.is_empty());

        // Match: insert on "users"
        let ev = make_event(SqliteChangeOp::Insert, "users", 1);
        assert!(d.entries[0].matches(&ev));

        // No match: update on "users"
        let ev2 = make_event(SqliteChangeOp::Update, "users", 1);
        assert!(!d.entries[0].matches(&ev2));

        // No match: insert on "posts"
        let ev3 = make_event(SqliteChangeOp::Insert, "posts", 1);
        assert!(!d.entries[0].matches(&ev3));

        // Remove
        assert!(d.remove(id));
        assert!(d.is_empty());

        // Double remove returns false
        assert!(!d.remove(id));
    }

    #[test]
    fn dispatcher_catch_all_matches_any_table() {
        let mut d = ChangeHookDispatcher::new();
        d.add(
            None, // catch-all
            SqliteChangeOps::ALL,
            Box::new(|_| {}),
        );

        let ev1 = make_event(SqliteChangeOp::Insert, "users", 1);
        let ev2 = make_event(SqliteChangeOp::Delete, "posts", 2);
        assert!(d.entries[0].matches(&ev1));
        assert!(d.entries[0].matches(&ev2));
    }

    #[test]
    fn dispatcher_dispatch_calls_matching_hooks() {
        let mut d = ChangeHookDispatcher::new();

        let fired = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let f2 = fired.clone();

        d.add(
            Some("users"),
            SqliteChangeOps::INSERT,
            Box::new(move |e| {
                f2.lock().unwrap().push(e.rowid);
            }),
        );

        d.dispatch(make_event(SqliteChangeOp::Insert, "users", 42));
        d.dispatch(make_event(SqliteChangeOp::Update, "users", 43)); // no match
        d.dispatch(make_event(SqliteChangeOp::Insert, "posts", 44)); // no match

        assert_eq!(*fired.lock().unwrap(), vec![42]);
    }

    #[test]
    fn dispatcher_clear_for_table() {
        let mut d = ChangeHookDispatcher::new();
        d.add(Some("users"), SqliteChangeOps::INSERT, Box::new(|_| {}));
        d.add(Some("posts"), SqliteChangeOps::INSERT, Box::new(|_| {}));
        d.add(
            None, // catch-all — should NOT be removed
            SqliteChangeOps::ALL,
            Box::new(|_| {}),
        );

        assert_eq!(d.entries.len(), 3);
        d.clear_for_table("users");
        assert_eq!(d.entries.len(), 2);
        // "posts" entry and catch-all remain
        assert_eq!(d.entries[0].table_name, Some("posts"));
        assert_eq!(d.entries[1].table_name, None);
    }

    #[test]
    fn dispatcher_clear_all() {
        let mut d = ChangeHookDispatcher::new();
        d.add(Some("a"), SqliteChangeOps::ALL, Box::new(|_| {}));
        d.add(Some("b"), SqliteChangeOps::ALL, Box::new(|_| {}));
        d.add(None, SqliteChangeOps::ALL, Box::new(|_| {}));
        assert_eq!(d.entries.len(), 3);
        d.clear_all();
        assert!(d.is_empty());
    }
}
