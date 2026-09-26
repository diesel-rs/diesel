//! Types used by [`SqliteConnection::on_collation_needed`](super::SqliteConnection::on_collation_needed).

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;

/// Text encoding SQLite requested for a missing collation.
///
/// [`register_collation`](super::SqliteConnection::register_collation) always
/// installs `SQLITE_UTF8`, so most callbacks can ignore this field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqliteTextRep {
    /// `SQLITE_UTF8`.
    Utf8,
    /// `SQLITE_UTF16BE`.
    Utf16Be,
    /// `SQLITE_UTF16LE`.
    Utf16Le,
    /// An encoding this release does not name. Preserves the raw
    /// `eTextRep` for forward compatibility. Treat the inner integer as
    /// opaque, and match a named variant if a future Diesel release adds one.
    Other(i32),
}

impl SqliteTextRep {
    pub(super) fn from_ffi(text_rep: i32) -> Self {
        match text_rep {
            ffi::SQLITE_UTF8 => SqliteTextRep::Utf8,
            ffi::SQLITE_UTF16BE => SqliteTextRep::Utf16Be,
            ffi::SQLITE_UTF16LE => SqliteTextRep::Utf16Le,
            other => SqliteTextRep::Other(other),
        }
    }
}

/// Context passed to the collation-needed callback.
///
/// Added in SQLite 3.0.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CollationNeededContext<'a> {
    /// The name of the missing collation, as UTF-8.
    pub name: &'a str,
    /// Preferred text encoding for the collation.
    pub text_rep: SqliteTextRep,
}

impl SqliteConnection {
    /// Registers a callback fired when SQLite encounters an unknown collation.
    ///
    /// The callback receives a borrowed `&mut SqliteConnection` and a
    /// [`CollationNeededContext`] naming the missing collation. It should
    /// install the missing collation via
    /// [`register_collation`](Self::register_collation) and return, after
    /// which SQLite retries the lookup. The callback body may also execute
    /// arbitrary SQL through `conn`, provided it leaves no open transaction.
    ///
    /// Only one callback is active at a time, and re-registering replaces it.
    /// Panics in the callback abort the process.
    ///
    /// If the callback registers a collation that is itself missing, SQLite
    /// re-enters the callback. Guard against unbounded recursion in that case.
    ///
    /// Added in SQLite 3.0.0.
    ///
    /// See: [`sqlite3_collation_needed`](https://www.sqlite.org/c3ref/collation_needed.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_collation_needed(|conn, ctx| {
    ///     if ctx.name.eq_ignore_ascii_case("RUSTNOCASE") {
    ///         let _ = conn.register_collation("RUSTNOCASE", |a, b| {
    ///             a.to_lowercase().cmp(&b.to_lowercase())
    ///         });
    ///     }
    /// });
    ///
    /// // Later: remove the callback
    /// conn.remove_collation_needed_hook();
    /// ```
    pub fn on_collation_needed<F>(&mut self, hook: F)
    where
        F: Fn(&mut SqliteConnection, CollationNeededContext<'_>) + Send + 'static,
    {
        self.raw_connection.set_collation_needed_hook(hook);
    }

    /// Removes the collation-needed callback.
    ///
    /// See [`on_collation_needed`](Self::on_collation_needed) for usage
    /// example.
    pub fn remove_collation_needed_hook(&mut self) {
        self.raw_connection.remove_collation_needed_hook();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::query_dsl::RunQueryDsl;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[diesel_test_helper::test]
    fn from_ffi_maps_all_documented_variants() {
        assert_eq!(
            SqliteTextRep::from_ffi(ffi::SQLITE_UTF8),
            SqliteTextRep::Utf8
        );
        assert_eq!(
            SqliteTextRep::from_ffi(ffi::SQLITE_UTF16BE),
            SqliteTextRep::Utf16Be,
        );
        assert_eq!(
            SqliteTextRep::from_ffi(ffi::SQLITE_UTF16LE),
            SqliteTextRep::Utf16Le,
        );
    }

    #[diesel_test_helper::test]
    fn from_ffi_preserves_unknown_encodings_in_other() {
        // 999 is not any current SQLite eTextRep. If SQLite ever assigns it,
        // this test starts failing and the enum learns a new named variant.
        assert_eq!(SqliteTextRep::from_ffi(999), SqliteTextRep::Other(999));
    }

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[diesel_test_helper::test]
    fn on_collation_needed_registration_is_safe() {
        let conn = &mut connection();

        conn.on_collation_needed(|_conn, _ctx| {});
        conn.remove_collation_needed_hook();

        crate::sql_query("SELECT 1").execute(conn).unwrap();
    }

    #[diesel_test_helper::test]
    fn replacing_collation_needed_hook_drops_old() {
        use std::sync::atomic::AtomicBool;

        let conn = &mut connection();

        let first_fired = Arc::new(AtomicBool::new(false));
        let first_fired2 = first_fired.clone();
        conn.on_collation_needed(move |_conn, _ctx| {
            first_fired2.store(true, Ordering::Relaxed);
        });

        let second_fired = Arc::new(AtomicBool::new(false));
        let second_fired2 = second_fired.clone();
        conn.on_collation_needed(move |conn, ctx| {
            conn.register_collation(ctx.name, |a, b| a.cmp(b)).unwrap();
            second_fired2.store(true, Ordering::Relaxed);
        });

        // `CREATE INDEX ... COLLATE FOO` forces SQLite to resolve FOO. A bare
        // `SELECT ... COLLATE FOO` does not.
        crate::sql_query("CREATE TABLE t_replace (x TEXT)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE INDEX i_replace ON t_replace (x COLLATE REPLACE_ME_COLL)")
            .execute(conn)
            .unwrap();

        assert!(
            !first_fired.load(Ordering::Relaxed),
            "the replaced hook must not have been invoked"
        );
        assert!(
            second_fired.load(Ordering::Relaxed),
            "the current hook should have fired"
        );
    }

    #[diesel_test_helper::test]
    fn collation_needed_fires_and_registers_collation() {
        use crate::sqlite::SqliteTextRep;
        use std::sync::atomic::AtomicBool;

        let conn = &mut connection();

        // The exact name is verified indirectly by `register_collation(ctx.name, ...)`.
        // If the callback saw the wrong name, the retry for MYCOLL would still fail.
        let fired = Arc::new(AtomicBool::new(false));
        let saw_name = Arc::new(AtomicBool::new(false));
        let saw_utf8 = Arc::new(AtomicBool::new(false));
        let fired2 = fired.clone();
        let saw_name2 = saw_name.clone();
        let saw_utf8_2 = saw_utf8.clone();

        conn.on_collation_needed(move |conn, ctx| {
            fired2.store(true, Ordering::Relaxed);
            if ctx.name == "MYCOLL" {
                saw_name2.store(true, Ordering::Relaxed);
            }
            if ctx.text_rep == SqliteTextRep::Utf8 {
                saw_utf8_2.store(true, Ordering::Relaxed);
            }
            conn.register_collation(ctx.name, |a, b| a.cmp(b)).unwrap();
        });

        // See `replacing_collation_needed_hook_drops_old` for why CREATE INDEX.
        crate::sql_query("CREATE TABLE t_fires (x TEXT)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE INDEX i_fires ON t_fires (x COLLATE MYCOLL)")
            .execute(conn)
            .unwrap();

        assert!(
            fired.load(Ordering::Relaxed),
            "collation_needed callback should fire"
        );
        assert!(
            saw_name.load(Ordering::Relaxed),
            "callback should observe the exact missing collation name"
        );
        assert!(
            saw_utf8.load(Ordering::Relaxed),
            "SQLite should request the UTF-8 encoding for a plain TEXT column"
        );
    }

    #[diesel_test_helper::test]
    fn remove_collation_needed_hook_without_registration_is_noop() {
        let conn = &mut connection();

        conn.remove_collation_needed_hook();
        crate::sql_query("SELECT 1").execute(conn).unwrap();
    }

    #[diesel_test_helper::test]
    fn callback_can_be_reentered_from_within_its_own_body() {
        use std::sync::atomic::AtomicBool;

        let conn = &mut connection();

        // Both flips must land: the outer callback fires for OUTER_COLL, and
        // then the SQL it executes internally triggers a second callback for
        // INNER_COLL while the outer callback frame is still on the stack.
        // This is the scenario the `Fn` (not `FnMut`) bound is designed to
        // support.
        let saw_outer = Arc::new(AtomicBool::new(false));
        let saw_inner = Arc::new(AtomicBool::new(false));
        let saw_outer2 = saw_outer.clone();
        let saw_inner2 = saw_inner.clone();

        conn.on_collation_needed(move |conn, ctx| {
            if ctx.name.eq_ignore_ascii_case("OUTER_COLL") {
                saw_outer2.store(true, Ordering::Relaxed);
                conn.register_collation("OUTER_COLL", |a, b| a.cmp(b))
                    .unwrap();
                // From inside our own frame, drive SQL that needs INNER_COLL,
                // which is also unregistered. SQLite must be able to call the
                // trampoline re-entrantly to satisfy this.
                crate::sql_query("CREATE TABLE t_reent_inner (x TEXT)")
                    .execute(conn)
                    .unwrap();
                crate::sql_query(
                    "CREATE INDEX i_reent_inner ON t_reent_inner (x COLLATE INNER_COLL)",
                )
                .execute(conn)
                .unwrap();
            } else if ctx.name.eq_ignore_ascii_case("INNER_COLL") {
                saw_inner2.store(true, Ordering::Relaxed);
                conn.register_collation("INNER_COLL", |a, b| a.cmp(b))
                    .unwrap();
            } else {
                panic!("unexpected collation name: {}", ctx.name);
            }
        });

        crate::sql_query("CREATE TABLE t_reent_outer (x TEXT)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE INDEX i_reent_outer ON t_reent_outer (x COLLATE OUTER_COLL)")
            .execute(conn)
            .unwrap();

        assert!(
            saw_outer.load(Ordering::Relaxed),
            "outer callback should fire for OUTER_COLL"
        );
        assert!(
            saw_inner.load(Ordering::Relaxed),
            "inner callback should fire re-entrantly from within the outer body"
        );
    }

    #[diesel_test_helper::test]
    fn remove_collation_needed_hook_stops_future_callbacks() {
        let conn = &mut connection();
        let calls: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        conn.on_collation_needed(move |conn, ctx| {
            calls2.fetch_add(1, Ordering::Relaxed);
            conn.register_collation(ctx.name, |a, b| a.cmp(b)).unwrap();
        });

        // First trigger: callback fires and installs MYCOLL_STOP.
        crate::sql_query("CREATE TABLE t_stop (x TEXT)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE INDEX i_stop ON t_stop (x COLLATE MYCOLL_STOP)")
            .execute(conn)
            .unwrap();
        let after_first = calls.load(Ordering::Relaxed);
        assert!(after_first > 0, "callback should fire while registered");

        conn.remove_collation_needed_hook();

        // Second trigger with a fresh unregistered collation: SQL must fail
        // (nothing left to install YOURCOLL_STOP) and the counter must not
        // move. A regression that only drops the Rust box while leaving the
        // C-side pointer registered would call into freed memory here.
        let result = crate::sql_query("CREATE INDEX i_stop2 ON t_stop (x COLLATE YOURCOLL_STOP)")
            .execute(conn);
        assert!(
            result.is_err(),
            "SQL referencing an unregistered collation should fail after remove"
        );
        assert_eq!(
            calls.load(Ordering::Relaxed),
            after_first,
            "callback must not fire after remove_collation_needed_hook"
        );
    }
}
