//! SQLite-specific connection extensions.

/// Trait for identifying a SQLite extension.
///
/// This trait acts as a "marker" for known, trusted extensions. By implementing this trait
/// for a zero-sized struct, you can use [`SqliteConnection::load_extension`] to safely load
/// that extension.
///
/// This design enforces two safety properties:
/// 1.  It prevents passing arbitrary user strings to the underlying loading mechanism, preventing
///     potential injection if user input were somehow involved (though `load_extension` itself
///     should never take user input).
/// 2.  It creates a catalog of known extensions in the codebase.
///
/// # Example
///
/// ```rust
/// use diesel::sqlite::SqliteExtension;
///
/// struct MyCryptoExtension;
///
/// impl SqliteExtension for MyCryptoExtension {
///     // The extension filename without 'lib' prefix or .so/.dll suffix
///     const FILENAME: &'static std::ffi::CStr = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"crypto\0") };
/// }
/// ```
pub trait SqliteExtension {
    /// The name of the extension library file (without the platform-specific extension like .dll or .so).
    /// We use a CStr here to ensure it is null-terminated for FFI calls, and we do not have to execute
    /// the potentially fallible conversion at runtime.
    const FILENAME: &'static std::ffi::CStr;
}

/// A marker struct for the UUID SQLite extension.
///
/// Using this struct with [`SqliteConnection::load_extension`] attempts to load
/// the "uuid" extension.
#[derive(Debug, Clone, Copy)]
pub struct SqliteUUIDExtension;

#[allow(unsafe_code)]
impl SqliteExtension for SqliteUUIDExtension {
    const FILENAME: &'static std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"uuid\0") };
}

/// A marker struct for the "extension-functions" SQLite extension.
///
/// This extension provides mathematical and string functions such as `sin()`, `cos()`, `power()`, `soundex()`, etc.
/// See the [SQLite Contrib](https://www.sqlite.org/contrib) page for details.
#[derive(Debug, Clone, Copy)]
pub struct SqliteMathFunctionsExtension;

#[allow(unsafe_code)]
impl SqliteExtension for SqliteMathFunctionsExtension {
    // Commonly named "libsqlitefunctions" or "extension-functions" depending on distribution.
    // We try "extension-functions" here as a reasonable default for the library name.
    const FILENAME: &'static std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"extension-functions\0") };
}

/// A marker struct for the "spellfix1" SQLite extension.
///
/// Provides the `spellfix1` virtual table for spell correction.
/// See [Spellfix1 documentation](https://www.sqlite.org/spellfix1.html).
#[derive(Debug, Clone, Copy)]
pub struct SqliteSpellfix1Extension;

#[allow(unsafe_code)]
impl SqliteExtension for SqliteSpellfix1Extension {
    const FILENAME: &'static std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"spellfix1\0") };
}
