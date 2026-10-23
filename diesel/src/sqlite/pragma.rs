//! Typed access to SQLite [pragmas](https://www.sqlite.org/pragma.html).
//!
//! A pragma is declared as a type implementing [`Pragma`], with [`ReadPragma`] when it returns rows and [`WritePragma`] when it can be assigned a value. Its [`Scope`](Pragma::Scope) decides whether it is used per database or per connection, and the caller picks the Rust type each row decodes into.
//!
//! # Per-schema pragmas
//!
//! A [`PerSchema`] pragma applies to one database and is read and written through [`SchemaPragma`], where a `None` schema selects `main`.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::sqlite::pragma::{
//!     PerSchema, Pragma, PragmaLiteral, ReadPragma, SchemaPragma, WritePragma,
//! };
//!
//! struct UserVersion(i32);
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
//! impl WritePragma for UserVersion {
//!     fn value(&self) -> PragmaLiteral<'_> {
//!         self.0.into()
//!     }
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! UserVersion(7).set(conn, None)?;
//! assert_eq!(7, UserVersion::get::<i32>(conn, None)?);
//! #     Ok(())
//! # }
//! ```
//!
//! # Per-connection pragmas
//!
//! A [`PerConnection`] pragma applies to the whole connection and is read and written through [`ConnectionPragma`], which takes no schema.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::connection::SimpleConnection;
//! use diesel::sqlite::pragma::{ConnectionPragma, PerConnection, Pragma, ReadPragma};
//!
//! struct ForeignKeys;
//!
//! impl Pragma for ForeignKeys {
//!     const NAME: &'static str = "foreign_keys";
//!     type Scope = PerConnection;
//! }
//!
//! impl ReadPragma for ForeignKeys {
//!     type SqlType = Bool;
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! conn.batch_execute("PRAGMA foreign_keys = ON")?;
//! assert!(ForeignKeys::get::<bool>(conn)?);
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
use alloc::string::ToString;
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

/// A pragma that can be assigned the value it carries.
///
/// Never assign secrets this way, since [`Instrumentation`](crate::connection::Instrumentation), [`on_trace`](SqliteConnection::on_trace) and [`on_authorize`](SqliteConnection::on_authorize) observe the statement.
pub trait WritePragma: Pragma {
    /// The literal assigned to the pragma.
    fn value(&self) -> PragmaLiteral<'_>;
}

/// A value as written into a pragma statement, which takes no bind parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PragmaLiteral<'a> {
    /// Rendered as a decimal integer.
    Integer(i64),
    /// Rendered unquoted, such as `WAL` or `NORMAL`.
    ///
    /// A keyword that is empty or holds anything but ASCII letters, digits and underscores fails with [`Error::QueryBuilderError`] before reaching SQLite.
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::sqlite::pragma::{
    ///     PerSchema, Pragma, PragmaLiteral, ReadPragma, SchemaPragma, WritePragma,
    /// };
    ///
    /// enum Synchronous {
    ///     Off,
    ///     Normal,
    /// }
    ///
    /// impl Pragma for Synchronous {
    ///     const NAME: &'static str = "synchronous";
    ///     type Scope = PerSchema;
    /// }
    ///
    /// impl ReadPragma for Synchronous {
    ///     type SqlType = Integer;
    /// }
    ///
    /// impl WritePragma for Synchronous {
    ///     fn value(&self) -> PragmaLiteral<'_> {
    ///         match self {
    ///             Synchronous::Off => PragmaLiteral::Keyword("OFF"),
    ///             Synchronous::Normal => PragmaLiteral::Keyword("NORMAL"),
    ///         }
    ///     }
    /// }
    ///
    /// # fn main() -> QueryResult<()> {
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// Synchronous::Normal.set(conn, None)?;
    /// assert_eq!(1, Synchronous::get::<i32>(conn, None)?);
    /// Synchronous::Off.set(conn, None)?;
    /// assert_eq!(0, Synchronous::get::<i32>(conn, None)?);
    /// #     Ok(())
    /// # }
    /// ```
    Keyword(&'static str),
    /// Rendered as a single-quoted SQL string with embedded quotes doubled, such as `'UTF-16le'`.
    ///
    /// Text holding a NUL byte fails with [`Error::InvalidCString`] before reaching SQLite.
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::sqlite::pragma::{
    ///     ConnectionPragma, PerConnection, Pragma, PragmaLiteral, ReadPragma, WritePragma,
    /// };
    ///
    /// struct Encoding<'a>(&'a str);
    ///
    /// impl Pragma for Encoding<'_> {
    ///     const NAME: &'static str = "encoding";
    ///     type Scope = PerConnection;
    /// }
    ///
    /// impl ReadPragma for Encoding<'_> {
    ///     type SqlType = Text;
    /// }
    ///
    /// impl WritePragma for Encoding<'_> {
    ///     fn value(&self) -> PragmaLiteral<'_> {
    ///         PragmaLiteral::Text(self.0)
    ///     }
    /// }
    ///
    /// # // Reading text allocated by the native sqlite library is not supported
    /// # // by miri `-Zmiri-native-lib`.
    /// # #[cfg(not(miri))]
    /// # fn main() -> QueryResult<()> {
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// // SQLite only changes the encoding before anything is written.
    /// Encoding("UTF-16le").set(conn)?;
    /// assert_eq!("UTF-16le", Encoding::get::<String>(conn)?);
    /// #     Ok(())
    /// # }
    /// # #[cfg(miri)]
    /// # fn main() {}
    /// ```
    Text(&'a str),
}

macro_rules! integer_pragma_literal {
    ($($ty:ty),*) => {$(
        impl From<$ty> for PragmaLiteral<'_> {
            fn from(value: $ty) -> Self {
                PragmaLiteral::Integer(i64::from(value))
            }
        }
    )*};
}

integer_pragma_literal!(i8, i16, i32, i64, u8, u16, u32, bool);

/// Reads and writes a [`PerSchema`] pragma, where a `None` schema selects `main`.
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

    /// Assign [`value`](WritePragma::value), running the statement to completion without confirming the value SQLite applied.
    fn set(&self, conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<()>
    where
        Self: WritePragma,
    {
        assign(conn, Some(schema.unwrap_or("main")), self)
    }
}

impl<P: Pragma<Scope = PerSchema>> SchemaPragma for P {}

/// Reads and writes a [`PerConnection`] pragma.
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

    /// Assign [`value`](WritePragma::value), running the statement to completion without confirming the value SQLite applied.
    fn set(&self, conn: &mut SqliteConnection) -> QueryResult<()>
    where
        Self: WritePragma,
    {
        assign(conn, None, self)
    }
}

impl<P: Pragma<Scope = PerConnection>> ConnectionPragma for P {}

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
    LoadConnection::<DefaultLoadingMode>::load(conn, PragmaQuery::<P>::new(schema, None))
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

fn assign<P: WritePragma>(
    conn: &mut SqliteConnection,
    schema: Option<&str>,
    pragma: &P,
) -> QueryResult<()> {
    use crate::connection::SimpleConnection;
    use crate::query_builder::QueryBuilder;
    use crate::sqlite::SqliteQueryBuilder;

    // Some pragmas do one unit of work per step, `incremental_vacuum` frees one page per step.
    // `sqlite3_exec` behind `batch_execute` runs to completion, while `StatementUse::run` steps once.
    let mut query = SqliteQueryBuilder::new();
    PragmaQuery::<P>::new(schema, Some(pragma.value())).to_sql(&mut query, &Sqlite)?;
    conn.batch_execute(&query.finish())
}

#[derive(Debug)]
struct PragmaQuery<'a, P> {
    schema: Option<&'a str>,
    value: Option<PragmaLiteral<'a>>,
    pragma: PhantomData<P>,
}

impl<'a, P: Pragma> PragmaQuery<'a, P> {
    fn new(schema: Option<&'a str>, value: Option<PragmaLiteral<'a>>) -> Self {
        Self {
            schema,
            value,
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
        push_bare_word(&mut out, "name", P::NAME)?;
        match self.value {
            None => {}
            Some(PragmaLiteral::Integer(value)) => {
                out.push_sql(" = ");
                out.push_sql(&value.to_string());
            }
            Some(PragmaLiteral::Keyword(keyword)) => {
                out.push_sql(" = ");
                push_bare_word(&mut out, "keyword", keyword)?;
            }
            Some(PragmaLiteral::Text(text)) => {
                out.push_sql(" = '");
                for (index, fragment) in text.split('\'').enumerate() {
                    if index != 0 {
                        out.push_sql("''");
                    }
                    out.push_sql(fragment);
                }
                out.push_sql("'");
            }
        }
        Ok(())
    }
}

fn push_bare_word(
    out: &mut AstPass<'_, '_, Sqlite>,
    kind: &str,
    word: &'static str,
) -> QueryResult<()> {
    if word.is_empty()
        || !word
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(Error::QueryBuilderError(
            alloc::format!("`{word}` is not a valid pragma {kind}").into(),
        ));
    }
    out.push_sql(word);
    Ok(())
}

// The schema name and value are runtime data, so the rendered SQL is not determined by the type.
impl<P> QueryId for PragmaQuery<'_, P> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<P: ReadPragma> Query for PragmaQuery<'_, P> {
    type SqlType = P::SqlType;
}
