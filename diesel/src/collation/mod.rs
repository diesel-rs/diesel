//! This module contains types that represent SQL collations.
//!
//! These types are used as arguments for
//! [`TextExpressionMethods::collate`](crate::expression_methods::TextExpressionMethods::collate).

use std::fmt;

/// A custom collation.
///
/// This type wraps a string that represents a collation name.
/// It can be used to use a collation that is not supported by Diesel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Custom(pub &'static str);

impl fmt::Display for Custom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The `BINARY` collation.
///
/// This collation is binary, case-sensitive, and locale-free.
/// It is supported by MySQL and SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(any(feature = "sqlite", feature = "mysql"))]
pub struct Binary;

#[cfg(any(feature = "sqlite", feature = "mysql"))]
impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BINARY")
    }
}

/// The `C` collation.
///
/// This collation is byte-wise, with only ASCII A-Z treated as letters.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct C;

#[cfg(feature = "postgres")]
impl fmt::Display for C {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"C\"")
    }
}

/// The `NOCASE` collation.
///
/// This collation is ASCII-only case-insensitive.
/// It is supported by SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "sqlite")]
pub struct NoCase;

#[cfg(feature = "sqlite")]
impl fmt::Display for NoCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NOCASE")
    }
}

/// The `POSIX` collation.
///
/// This collation is byte-wise, with only ASCII A-Z treated as letters.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct Posix;

#[cfg(feature = "postgres")]
impl fmt::Display for Posix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "POSIX")
    }
}

/// The `RTRIM` collation.
///
/// This collation ignores trailing spaces.
/// It is supported by SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "sqlite")]
pub struct RTrim;

#[cfg(feature = "sqlite")]
impl fmt::Display for RTrim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RTRIM")
    }
}

/// The `unicode` collation.
///
/// This collation uses the Unicode Collation Algorithm for natural language order.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct Unicode;

#[cfg(feature = "postgres")]
impl fmt::Display for Unicode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unicode")
    }
}

/// The `ucs_basic` collation.
///
/// This collation uses Unicode code-point ordering with no linguistic rules.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct UcsBasic;

#[cfg(feature = "postgres")]
impl fmt::Display for UcsBasic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ucs_basic")
    }
}

/// The `pg_unicode_fast` collation.
///
/// This collation uses code-point ordering with full Unicode case mapping.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct PgUnicodeFast;

#[cfg(feature = "postgres")]
impl fmt::Display for PgUnicodeFast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pg_unicode_fast")
    }
}

/// The `pg_c_utf8` collation.
///
/// This collation uses code-point ordering with simple case mapping.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct PgCUtf8;

#[cfg(feature = "postgres")]
impl fmt::Display for PgCUtf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pg_c_utf8")
    }
}

/// The `default` collation.
///
/// This collation uses the database locale at creation time.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct Default;

#[cfg(feature = "postgres")]
impl fmt::Display for Default {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("default")
    }
}
