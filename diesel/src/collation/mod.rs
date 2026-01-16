//! This module contains types that represent SQL collations.
//!
//! These types are used as arguments for
//! [`TextExpressionMethods::collate`](crate::expression_methods::TextExpressionMethods::collate).

use crate::backend::Backend;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::result::QueryResult;

/// Trait to identify a valid collation.
pub trait Collation: QueryId + Copy + Send + Sync + 'static {}

/// A custom collation.
///
/// This type wraps a string that represents a collation name.
/// It can be used to use a collation that is not supported by Diesel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Custom(pub &'static str);

impl Collation for Custom {}

impl<DB: Backend> QueryFragment<DB> for Custom {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(self.0);
        Ok(())
    }
}

impl QueryId for Custom {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

/// The `BINARY` collation.
///
/// This collation is binary, case-sensitive, and locale-free.
/// It is supported by SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "sqlite")]
pub struct Binary;

#[cfg(feature = "sqlite")]
impl Collation for Binary {}

#[cfg(feature = "sqlite")]
impl QueryFragment<crate::sqlite::Sqlite> for Binary {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        out.push_sql("BINARY");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryId for Binary {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

/// The `C` collation.
///
/// This collation is byte-wise, with only ASCII A-Z treated as letters.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct C;

#[cfg(feature = "postgres")]
impl Collation for C {}

#[cfg(feature = "postgres")]
impl QueryFragment<crate::pg::Pg> for C {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::pg::Pg>) -> QueryResult<()> {
        out.push_sql("\"C\"");
        Ok(())
    }
}

#[cfg(feature = "postgres")]
impl QueryId for C {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

/// The `NOCASE` collation.
///
/// This collation is ASCII-only case-insensitive.
/// It is supported by SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "sqlite")]
pub struct NoCase;

#[cfg(feature = "sqlite")]
impl Collation for NoCase {}

#[cfg(feature = "sqlite")]
impl QueryFragment<crate::sqlite::Sqlite> for NoCase {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        out.push_sql("NOCASE");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryId for NoCase {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

/// The `POSIX` collation.
///
/// This collation is byte-wise, with only ASCII A-Z treated as letters.
/// It is supported by PostgreSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "postgres")]
pub struct Posix;

#[cfg(feature = "postgres")]
impl Collation for Posix {}

#[cfg(feature = "postgres")]
impl QueryFragment<crate::pg::Pg> for Posix {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::pg::Pg>) -> QueryResult<()> {
        out.push_sql("\"POSIX\"");
        Ok(())
    }
}

#[cfg(feature = "postgres")]
impl QueryId for Posix {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

/// The `RTRIM` collation.
///
/// This collation ignores trailing spaces.
/// It is supported by SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg(feature = "sqlite")]
pub struct RTrim;

#[cfg(feature = "sqlite")]
impl Collation for RTrim {}

#[cfg(feature = "sqlite")]
impl QueryFragment<crate::sqlite::Sqlite> for RTrim {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, crate::sqlite::Sqlite>) -> QueryResult<()> {
        out.push_sql("RTRIM");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl QueryId for RTrim {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}
