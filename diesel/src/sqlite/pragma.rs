//! Typed access to SQLite [pragmas](https://www.sqlite.org/pragma.html).
//!
//! A pragma is declared as a type implementing [`Pragma`], and [`ReadPragma`] when it returns rows. Its [`Scope`](Pragma::Scope) decides whether it is read per database or per connection, and the caller picks the Rust type each row decodes into.
//!
//! # Per-schema pragmas
//!
//! A [`PerSchema`] pragma applies to one database and is read through [`SchemaPragma`], where a `None` schema selects `main`.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::connection::SimpleConnection;
//! use diesel::sqlite::pragma::{PerSchema, Pragma, ReadPragma, SchemaPragma};
//!
//! struct UserVersion;
//!
//! impl Pragma for UserVersion {
//!     const NAME: &'static str = "user_version";
//!     type Scope = PerSchema;
//! }
//!
//! impl ReadPragma for UserVersion {
//!     type SqlType = Integer;
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! conn.batch_execute("PRAGMA user_version = 7")?;
//! assert_eq!(7, UserVersion::get::<i32>(conn, None)?);
//! #     Ok(())
//! # }
//! ```
//!
//! # Per-connection pragmas
//!
//! A [`PerConnection`] pragma applies to the whole connection and is read through [`ConnectionPragma`], which takes no schema.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::sqlite::pragma::{ConnectionPragma, PerConnection, Pragma, ReadPragma};
//!
//! struct DatabaseList;
//!
//! impl Pragma for DatabaseList {
//!     const NAME: &'static str = "database_list";
//!     type Scope = PerConnection;
//! }
//!
//! impl ReadPragma for DatabaseList {
//!     type SqlType = (Integer, Text, Text);
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! let databases = DatabaseList::get_all::<(i32, String, String)>(conn)?;
//! assert_eq!("main", databases[0].1);
//! #     Ok(())
//! # }
//! ```

use crate::connection::{DefaultLoadingMode, LoadConnection};
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::result::{Error, QueryResult};
use crate::sql_types::SqlType;
use crate::sqlite::{Sqlite, SqliteConnection};
use alloc::vec::Vec;
use core::marker::PhantomData;

/// A pragma, named by [`NAME`](Self::NAME) and scoped by [`Scope`](Self::Scope).
pub trait Pragma: Sized {
    /// The pragma name, written into the SQL unquoted.
    ///
    /// A name that is empty or holds anything but ASCII letters, digits and underscores fails with [`Error::QueryBuilderError`] before reaching SQLite. Naming a pragma SQLite knows is up to the implementer, since SQLite ignores unknown ones.
    const NAME: &'static str;
    /// [`PerSchema`] or [`PerConnection`].
    type Scope: PragmaScope;
}

/// Whether a pragma targets one database or the whole connection.
pub trait PragmaScope: private::Sealed {}

/// A pragma that applies to one database, `main` or an attached one.
#[derive(Debug, Clone, Copy)]
pub enum PerSchema {}

/// A connection-wide pragma, whose schema qualifier SQLite ignores.
#[derive(Debug, Clone, Copy)]
pub enum PerConnection {}

impl PragmaScope for PerSchema {}
impl PragmaScope for PerConnection {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::PerSchema {}
    impl Sealed for super::PerConnection {}
}

/// A pragma that returns rows when queried.
pub trait ReadPragma: Pragma {
    /// The SQL type of one returned row.
    type SqlType: SqlType;
}

/// Reads a [`PerSchema`] pragma, where a `None` schema selects `main`.
pub trait SchemaPragma: Pragma<Scope = PerSchema> {
    /// Read the first row, returning [`Error::NotFound`] if the pragma returns none.
    fn get<Row>(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Row>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
        Row: FromSqlRow<Self::SqlType, Sqlite>,
    {
        first_row::<Self, Row>(conn, Some(schema.unwrap_or("main")))
    }

    /// Read every row.
    fn get_all<Row>(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Vec<Row>>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
        Row: FromSqlRow<Self::SqlType, Sqlite>,
    {
        all_rows::<Self, Row>(conn, Some(schema.unwrap_or("main")))
    }
}

impl<P: Pragma<Scope = PerSchema>> SchemaPragma for P {}

/// Reads a [`PerConnection`] pragma.
pub trait ConnectionPragma: Pragma<Scope = PerConnection> {
    /// Read the first row, returning [`Error::NotFound`] if the pragma returns none.
    fn get<Row>(conn: &mut SqliteConnection) -> QueryResult<Row>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
        Row: FromSqlRow<Self::SqlType, Sqlite>,
    {
        first_row::<Self, Row>(conn, None)
    }

    /// Read every row.
    fn get_all<Row>(conn: &mut SqliteConnection) -> QueryResult<Vec<Row>>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
        Row: FromSqlRow<Self::SqlType, Sqlite>,
    {
        all_rows::<Self, Row>(conn, None)
    }
}

impl<P: Pragma<Scope = PerConnection>> ConnectionPragma for P {}

/// `PRAGMA auto_vacuum`, the [`AutoVacuumMode`](crate::sqlite::AutoVacuumMode) of a database.
pub(crate) struct AutoVacuum;

impl Pragma for AutoVacuum {
    const NAME: &'static str = "auto_vacuum";
    type Scope = PerSchema;
}

impl ReadPragma for AutoVacuum {
    type SqlType = crate::sql_types::Integer;
}

/// `PRAGMA page_count`, the total number of pages in a database, read as an `i64`.
///
/// Multiply by the page size for the size the database accounts for.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel::connection::SimpleConnection;
/// use diesel::sqlite::pragma::{PageCount, SchemaPragma};
///
/// # fn main() -> QueryResult<()> {
/// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
/// // An empty database occupies no pages until something is written.
/// assert_eq!(0, PageCount::get::<i64>(conn, None)?);
/// conn.batch_execute("CREATE TABLE items (id INTEGER PRIMARY KEY)")?;
/// assert!(PageCount::get::<i64>(conn, None)? > 0);
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PageCount;

impl Pragma for PageCount {
    const NAME: &'static str = "page_count";
    type Scope = PerSchema;
}

impl ReadPragma for PageCount {
    type SqlType = crate::sql_types::BigInt;
}

/// `PRAGMA freelist_count`, the unused pages on a database's freelist, read as an `i64`.
///
/// A growing freelist is reclaimable space, freed by `VACUUM`.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel::sqlite::pragma::{FreelistCount, SchemaPragma};
///
/// # fn main() -> QueryResult<()> {
/// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
/// assert_eq!(0, FreelistCount::get::<i64>(conn, None)?);
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FreelistCount;

impl Pragma for FreelistCount {
    const NAME: &'static str = "freelist_count";
    type Scope = PerSchema;
}

impl ReadPragma for FreelistCount {
    type SqlType = crate::sql_types::BigInt;
}

fn cursor<'conn, 'query, P>(
    conn: &'conn mut SqliteConnection,
    schema: Option<&'query str>,
) -> QueryResult<<SqliteConnection as LoadConnection<DefaultLoadingMode>>::Cursor<'conn, 'query>>
where
    P: ReadPragma + 'query,
    Sqlite: QueryMetadata<P::SqlType>,
{
    LoadConnection::<DefaultLoadingMode>::load(conn, PragmaQuery::<P>::new(schema))
}

fn first_row<P, Row>(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Row>
where
    P: ReadPragma,
    Sqlite: QueryMetadata<P::SqlType>,
    Row: FromSqlRow<P::SqlType, Sqlite>,
{
    let mut cursor = cursor::<P>(conn, schema)?;
    let row = cursor.next().ok_or(Error::NotFound)??;
    Row::build_from_row(&row).map_err(Error::DeserializationError)
}

fn all_rows<P, Row>(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Vec<Row>>
where
    P: ReadPragma,
    Sqlite: QueryMetadata<P::SqlType>,
    Row: FromSqlRow<P::SqlType, Sqlite>,
{
    cursor::<P>(conn, schema)?
        .map(|row| Row::build_from_row(&row?).map_err(Error::DeserializationError))
        .collect()
}

#[derive(Debug)]
struct PragmaQuery<'a, P> {
    schema: Option<&'a str>,
    pragma: PhantomData<P>,
}

impl<'a, P: Pragma> PragmaQuery<'a, P> {
    fn new(schema: Option<&'a str>) -> Self {
        Self {
            schema,
            pragma: PhantomData,
        }
    }
}

impl<P: Pragma> QueryFragment<Sqlite> for PragmaQuery<'_, P> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("PRAGMA ");
        if let Some(schema) = self.schema {
            out.push_identifier(schema)?;
            out.push_sql(".");
        }
        let name = P::NAME;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(Error::QueryBuilderError(
                alloc::format!("`{name}` is not a valid pragma name").into(),
            ));
        }
        out.push_sql(name);
        Ok(())
    }
}

// The schema name is runtime data, so the rendered SQL is not determined by the type.
impl<P> QueryId for PragmaQuery<'_, P> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<P: ReadPragma> Query for PragmaQuery<'_, P> {
    type SqlType = P::SqlType;
}
