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
/// It is supported by SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "sqlite")]
pub struct Binary;

#[cfg(feature = "sqlite")]
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
        write!(f, "\"POSIX\"")
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
