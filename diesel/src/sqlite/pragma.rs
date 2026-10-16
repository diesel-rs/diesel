//! Typed access to SQLite [pragmas](https://www.sqlite.org/pragma.html).
//!
//! Never pass secrets as pragma values because [`Instrumentation`](crate::connection::Instrumentation), [`on_trace`](SqliteConnection::on_trace) and [`on_authorize`](SqliteConnection::on_authorize) can observe them.
//!
//! A pragma scoped [`PerSchema`] is read and written through [`SchemaPragma`], one scoped [`PerConnection`] through [`ConnectionPragma`].
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::sqlite::pragma::{
//!     ConnectionPragma, PerConnection, PerSchema, Pragma, PragmaLiteral, ReadPragma,
//!     SchemaPragma, WritePragma,
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
//!     type Row = i32;
//! }
//!
//! impl WritePragma for UserVersion {
//!     fn value(&self) -> PragmaLiteral<'_> {
//!         self.0.into()
//!     }
//! }
//!
//! struct ForeignKeys(bool);
//!
//! impl Pragma for ForeignKeys {
//!     const NAME: &'static str = "foreign_keys";
//!     type Scope = PerConnection;
//! }
//!
//! impl ReadPragma for ForeignKeys {
//!     type SqlType = Bool;
//!     type Row = bool;
//! }
//!
//! impl WritePragma for ForeignKeys {
//!     fn value(&self) -> PragmaLiteral<'_> {
//!         self.0.into()
//!     }
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! UserVersion(7).set(conn, None)?;
//! assert_eq!(7, UserVersion::get(conn, None)?);
//!
//! ForeignKeys(true).set(conn)?;
//! assert!(ForeignKeys::get(conn)?);
//! #     Ok(())
//! # }
//! ```
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::connection::SimpleConnection;
//! use diesel::sqlite::pragma::table_info;
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! let table = String::from("author's books");
//! conn.batch_execute(
//!     "CREATE TABLE \"author's books\" (id INTEGER PRIMARY KEY, title TEXT NOT NULL)"
//! )?;
//!
//! type Column = (i32, String, String, bool, Option<String>, i32);
//! let columns = table_info(&table, None).load::<Column>(conn)?;
//! assert_eq!(
//!     columns.iter().map(|column| (
//!         column.1.as_str(), column.2.as_str(), column.3, column.5
//!     )).collect::<Vec<_>>(),
//!     [("id", "INTEGER", false, 1), ("title", "TEXT", true, 0)],
//! );
//! # Ok(())
//! # }
//! ```

use crate::connection::{DefaultLoadingMode, LoadConnection};
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::query_dsl::RunQueryDslSupport;
use crate::result::{Error, QueryResult};
use crate::row::Row;
use crate::sql_types::{Bool, Integer, Nullable, SqlType, Text};
use crate::sqlite::{AutoVacuumMode, Sqlite, SqliteConnection};
use alloc::string::ToString;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// A pragma, named by [`NAME`](Self::NAME) and scoped by [`Scope`](Self::Scope).
pub trait Pragma: Sized {
    /// A nonempty name of ASCII letters, digits, or underscores.
    const NAME: &'static str;
    /// [`PerSchema`] or [`PerConnection`].
    type Scope: PragmaScope;
}

/// Whether a pragma targets one database or the whole connection.
pub trait PragmaScope: private::Sealed {}

/// A pragma that applies to one database, `main` or an attached one.
#[derive(Debug, Clone, Copy)]
pub enum PerSchema {}

/// A connection-wide pragma whose schema qualifier SQLite ignores.
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
    /// The Rust type one returned row decodes into.
    type Row: FromSqlRow<Self::SqlType, Sqlite>;
}

/// A pragma that can be assigned the value it carries, running to completion and discarding returned rows.
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
    /// A nonempty ASCII word of letters, digits, or underscores, rendered without quoting.
    Keyword(&'static str),
    /// Borrowed text rendered as a single-quoted literal with embedded quotes escaped and NUL bytes rejected.
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

impl From<AutoVacuumMode> for PragmaLiteral<'_> {
    fn from(mode: AutoVacuumMode) -> Self {
        PragmaLiteral::Integer(i64::from(mode as i32))
    }
}

/// Reads and writes a [`PerSchema`] pragma, where `None` selects `main`.
pub trait SchemaPragma: Pragma<Scope = PerSchema> {
    /// Read the first row, returning [`Error::NotFound`] if the pragma returns none.
    fn get(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Self::Row>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
    {
        first_row::<Self>(conn, Some(schema.unwrap_or("main")))
    }

    /// Read every row.
    fn get_all(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Vec<Self::Row>>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
    {
        all_rows::<Self>(conn, Some(schema.unwrap_or("main")))
    }

    /// Assign [`value`](WritePragma::value) without confirming the value SQLite applied.
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
    fn get(conn: &mut SqliteConnection) -> QueryResult<Self::Row>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
    {
        first_row::<Self>(conn, None)
    }

    /// Read every row.
    fn get_all(conn: &mut SqliteConnection) -> QueryResult<Vec<Self::Row>>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
    {
        all_rows::<Self>(conn, None)
    }

    /// Assign [`value`](WritePragma::value) without confirming the value SQLite applied.
    fn set(&self, conn: &mut SqliteConnection) -> QueryResult<()>
    where
        Self: WritePragma,
    {
        assign(conn, None, self)
    }
}

impl<P: Pragma<Scope = PerConnection>> ConnectionPragma for P {}

/// SQL types of `(cid, name, type, notnull, dflt_value, pk)`, where `pk` is the primary-key ordinal or zero.
pub type TableInfoSqlType = (Integer, Text, Text, Bool, Nullable<Text>, Integer);

/// A query for ordinary columns from `PRAGMA table_info`.
#[derive(Debug)]
pub struct TableInfoQuery<'a> {
    query: PragmaQuery<'a, TableInfoPragma>,
}

/// Reads column metadata for `table`, leaving SQLite's schema lookup unqualified when `schema` is `None`.
pub fn table_info<'a>(table: &'a str, schema: Option<&'a str>) -> TableInfoQuery<'a> {
    TableInfoQuery {
        query: PragmaQuery::new(schema, Some(PragmaLiteral::Text(table))),
    }
}

impl QueryFragment<Sqlite> for TableInfoQuery<'_> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.query.walk_ast(out)
    }
}

impl Query for TableInfoQuery<'_> {
    type SqlType = TableInfoSqlType;
}

impl QueryId for TableInfoQuery<'_> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl RunQueryDslSupport for TableInfoQuery<'_> {}

#[derive(Debug)]
struct TableInfoPragma;

impl Pragma for TableInfoPragma {
    const NAME: &'static str = "table_info";
    type Scope = PerSchema;
}

/// `PRAGMA auto_vacuum`, read as an [`AutoVacuumMode`] and set to the one it carries.
pub(crate) struct AutoVacuum(pub(crate) AutoVacuumMode);

impl Pragma for AutoVacuum {
    const NAME: &'static str = "auto_vacuum";
    type Scope = PerSchema;
}

impl ReadPragma for AutoVacuum {
    type SqlType = crate::sql_types::Integer;
    type Row = AutoVacuumMode;
}

impl WritePragma for AutoVacuum {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
}

/// `PRAGMA page_count`, the total number of pages in a database.
pub(crate) struct PageCount;

impl Pragma for PageCount {
    const NAME: &'static str = "page_count";
    type Scope = PerSchema;
}

impl ReadPragma for PageCount {
    type SqlType = crate::sql_types::BigInt;
    type Row = i64;
}

/// `PRAGMA freelist_count`, the unused pages on a database's freelist.
pub(crate) struct FreelistCount;

impl Pragma for FreelistCount {
    const NAME: &'static str = "freelist_count";
    type Scope = PerSchema;
}

impl ReadPragma for FreelistCount {
    type SqlType = crate::sql_types::BigInt;
    type Row = i64;
}

/// `PRAGMA incremental_vacuum`, freeing up to the carried number of freelist pages, all of them for 0.
pub(crate) struct IncrementalVacuum(pub(crate) u32);

impl Pragma for IncrementalVacuum {
    const NAME: &'static str = "incremental_vacuum";
    type Scope = PerSchema;
}

impl WritePragma for IncrementalVacuum {
    fn value(&self) -> PragmaLiteral<'_> {
        self.0.into()
    }
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

fn first_row<P>(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<P::Row>
where
    P: ReadPragma,
    Sqlite: QueryMetadata<P::SqlType>,
{
    let mut cursor = cursor::<P>(conn, schema)?;
    let row = cursor.next().ok_or(Error::NotFound)??;
    decode_row::<P>(&row)
}

fn all_rows<P>(conn: &mut SqliteConnection, schema: Option<&str>) -> QueryResult<Vec<P::Row>>
where
    P: ReadPragma,
    Sqlite: QueryMetadata<P::SqlType>,
{
    cursor::<P>(conn, schema)?
        .map(|row| decode_row::<P>(&row?))
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

fn decode_row<'a, P: ReadPragma>(row: &impl Row<'a, Sqlite>) -> QueryResult<P::Row> {
    P::Row::build_from_row(row).map_err(Error::DeserializationError)
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

// The schema name and value are runtime data, so the rendered SQL is not determined by the type.
impl<P> QueryId for PragmaQuery<'_, P> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<P: ReadPragma> Query for PragmaQuery<'_, P> {
    type SqlType = P::SqlType;
}

impl<P> RunQueryDslSupport for PragmaQuery<'_, P> {}

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
