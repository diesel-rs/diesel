//! Types used by [`SqliteConnection::on_collation_needed`](super::SqliteConnection::on_collation_needed).

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
