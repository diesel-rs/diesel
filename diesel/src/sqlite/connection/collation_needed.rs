//! Types used by [`SqliteConnection::on_collation_needed`](super::SqliteConnection::on_collation_needed).

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;

/// Text encoding SQLite requested for a missing collation.
///
/// Diesel registers the comparator a callback returns for `SQLITE_UTF8`, so most callbacks can ignore this field.
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
    /// The callback receives a [`CollationNeededContext`] naming the missing collation and returns its comparator, which Diesel registers under that name before SQLite retries the lookup, while `None` leaves it missing so the statement fails.
    ///
    /// Only one callback is active at a time, and re-registering replaces it.
    /// Panics in the callback abort the process.
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
    /// conn.on_collation_needed(|ctx| {
    ///     if ctx.name.eq_ignore_ascii_case("RUSTNOCASE") {
    ///         Some(Box::new(|a: &str, b: &str| {
    ///             a.to_lowercase().cmp(&b.to_lowercase())
    ///         }))
    ///     } else {
    ///         None
    ///     }
    /// });
    ///
    /// // Later: remove the callback
    /// conn.remove_collation_needed_hook();
    /// ```
    pub fn on_collation_needed<F>(&mut self, hook: F)
    where
        F: Fn(
                CollationNeededContext<'_>,
            ) -> Option<
                Box<dyn Fn(&str, &str) -> core::cmp::Ordering + Send + core::panic::UnwindSafe>,
            > + Send
            + 'static,
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

// miri doesn't support callbacks yet
#[cfg(all(test, not(miri)))]
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

    fn binary_order()
    -> Option<Box<dyn Fn(&str, &str) -> core::cmp::Ordering + Send + core::panic::UnwindSafe>> {
        Some(Box::new(|a: &str, b: &str| a.cmp(b)))
    }

    #[diesel_test_helper::test]
    fn on_collation_needed_registration_is_safe() {
        let conn = &mut connection();

        conn.on_collation_needed(|_ctx| None);
        conn.remove_collation_needed_hook();

        crate::sql_query("SELECT 1").execute(conn).unwrap();
    }

    #[diesel_test_helper::test]
    fn replacing_collation_needed_hook_drops_old() {
        use std::sync::atomic::AtomicBool;

        let conn = &mut connection();

        let first_fired = Arc::new(AtomicBool::new(false));
        let first_fired2 = first_fired.clone();
        conn.on_collation_needed(move |_ctx| {
            first_fired2.store(true, Ordering::Relaxed);
            None
        });

        let second_fired = Arc::new(AtomicBool::new(false));
        let second_fired2 = second_fired.clone();
        conn.on_collation_needed(move |_ctx| {
            second_fired2.store(true, Ordering::Relaxed);
            binary_order()
        });

        // `CREATE INDEX ... COLLATE FOO` forces SQLite to resolve FOO.
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

        let fired = Arc::new(AtomicBool::new(false));
        let saw_name = Arc::new(AtomicBool::new(false));
        let saw_utf8 = Arc::new(AtomicBool::new(false));
        let fired2 = fired.clone();
        let saw_name2 = saw_name.clone();
        let saw_utf8_2 = saw_utf8.clone();

        conn.on_collation_needed(move |ctx| {
            fired2.store(true, Ordering::Relaxed);
            if ctx.name == "MYCOLL" {
                saw_name2.store(true, Ordering::Relaxed);
            }
            if ctx.text_rep == SqliteTextRep::Utf8 {
                saw_utf8_2.store(true, Ordering::Relaxed);
            }
            binary_order()
        });

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
    fn returned_comparator_is_used_for_the_requested_collation() {
        use crate::collation::Custom;
        use crate::prelude::*;

        crate::table! {
            words (id) {
                id -> Integer,
                word -> Text,
            }
        }

        let conn = &mut connection();
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();
        conn.on_collation_needed(move |ctx| {
            calls2.fetch_add(1, Ordering::Relaxed);
            (ctx.name == "MYNOCASE").then(|| {
                Box::new(|a: &str, b: &str| a.to_lowercase().cmp(&b.to_lowercase()))
                    as Box<
                        dyn Fn(&str, &str) -> core::cmp::Ordering + Send + core::panic::UnwindSafe,
                    >
            })
        });

        crate::sql_query("CREATE TABLE words (id INTEGER PRIMARY KEY, word TEXT NOT NULL)")
            .execute(conn)
            .unwrap();
        crate::insert_into(words::table)
            .values([words::word.eq("a"), words::word.eq("b")])
            .execute(conn)
            .unwrap();

        let matching_a = || {
            words::table
                .select(words::word)
                .filter(words::word.collate(Custom("MYNOCASE")).eq("A"))
        };
        assert_eq!(matching_a().load::<String>(conn).unwrap(), ["a"]);
        assert_eq!(matching_a().load::<String>(conn).unwrap(), ["a"]);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            1,
            "the registered comparator must satisfy later lookups"
        );
    }

    #[diesel_test_helper::test]
    fn returning_none_leaves_the_collation_missing() {
        let conn = &mut connection();
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();
        conn.on_collation_needed(move |_ctx| {
            calls2.fetch_add(1, Ordering::Relaxed);
            None
        });

        crate::sql_query("CREATE TABLE t_none (x TEXT)")
            .execute(conn)
            .unwrap();
        let result = crate::sql_query("CREATE INDEX i_none ON t_none (x COLLATE UNKNOWN_COLL)")
            .execute(conn);

        assert!(
            result.is_err(),
            "a missing collation must fail the statement"
        );
        assert!(
            calls.load(Ordering::Relaxed) > 0,
            "the callback should be consulted"
        );
    }

    #[diesel_test_helper::test]
    fn remove_collation_needed_hook_without_registration_is_noop() {
        let conn = &mut connection();

        conn.remove_collation_needed_hook();
        crate::sql_query("SELECT 1").execute(conn).unwrap();
    }

    #[diesel_test_helper::test]
    fn remove_collation_needed_hook_stops_future_callbacks() {
        let conn = &mut connection();
        let calls: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        conn.on_collation_needed(move |_ctx| {
            calls2.fetch_add(1, Ordering::Relaxed);
            binary_order()
        });

        crate::sql_query("CREATE TABLE t_stop (x TEXT)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE INDEX i_stop ON t_stop (x COLLATE MYCOLL_STOP)")
            .execute(conn)
            .unwrap();
        let after_first = calls.load(Ordering::Relaxed);
        assert!(after_first > 0, "callback should fire while registered");

        conn.remove_collation_needed_hook();

        // A stale C-side registration would call into the dropped closure here.
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
