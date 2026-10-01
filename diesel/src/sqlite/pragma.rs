//! Typed access to SQLite [pragmas](https://www.sqlite.org/pragma.html).
//!
//! Never pass secrets as pragma values because [`Instrumentation`](crate::connection::Instrumentation), [`on_trace`](SqliteConnection::on_trace) and [`on_authorize`](SqliteConnection::on_authorize) can observe them.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::sqlite::pragma::{Pragma, PerSchema, ReadPragma, WritePragma};
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
//!     type Row = i32;
//! }
//!
//! impl WritePragma for UserVersion {
//!     type Value = i32;
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! conn.set_pragma::<UserVersion>(None, &7)?;
//! assert_eq!(7, conn.pragma::<UserVersion>(None)?);
//! #     Ok(())
//! # }
//! ```

use crate::connection::{DefaultLoadingMode, LoadConnection};
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::query_dsl::{RunQueryDsl, RunQueryDslSupport};
use crate::result::{Error, QueryResult};
use crate::row::Row;
use crate::sql_types::SqlType;
use crate::sqlite::{AutoVacuumMode, Sqlite, SqliteConnection};
use alloc::string::ToString;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// A pragma, named by [`NAME`](Self::NAME) and scoped by [`Scope`](Self::Scope).
pub trait Pragma: 'static {
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

/// A pragma that can be assigned a value.
pub trait WritePragma: Pragma {
    /// The value assigned to the pragma.
    type Value: ToPragmaLiteral;
}

/// A value as written into a pragma statement, which takes no bind parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PragmaLiteral {
    /// Rendered as a decimal integer.
    Integer(i64),
    /// A nonempty ASCII word of letters, digits, or underscores, rendered without quoting.
    Keyword(&'static str),
}

/// Converts a value into the literal a pragma statement carries.
pub trait ToPragmaLiteral {
    /// The literal for this value.
    fn to_pragma_literal(&self) -> PragmaLiteral;
}

macro_rules! integer_pragma_literal {
    ($($ty:ty),*) => {$(
        impl ToPragmaLiteral for $ty {
            fn to_pragma_literal(&self) -> PragmaLiteral {
                PragmaLiteral::Integer(i64::from(*self))
            }
        }
    )*};
}

integer_pragma_literal!(i8, i16, i32, i64, u8, u16, u32, bool);

impl ToPragmaLiteral for AutoVacuumMode {
    fn to_pragma_literal(&self) -> PragmaLiteral {
        PragmaLiteral::Integer(i64::from(*self as i32))
    }
}

/// `PRAGMA auto_vacuum`, read and set as an [`AutoVacuumMode`].
pub(crate) struct AutoVacuum;

impl Pragma for AutoVacuum {
    const NAME: &'static str = "auto_vacuum";
    type Scope = PerSchema;
}

impl ReadPragma for AutoVacuum {
    type SqlType = crate::sql_types::Integer;
    type Row = AutoVacuumMode;
}

impl WritePragma for AutoVacuum {
    type Value = AutoVacuumMode;
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

impl SqliteConnection {
    /// Read the first row of a per-schema pragma (`None` selects `main`), returning [`Error::NotFound`] if empty.
    pub fn pragma<P>(&mut self, schema: Option<&str>) -> QueryResult<P::Row>
    where
        P: ReadPragma<Scope = PerSchema>,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        self.first_pragma_row::<P>(Some(schema.unwrap_or("main")))
    }

    /// Read every row of a per-schema pragma (`None` selects `main`).
    pub fn pragma_rows<P>(&mut self, schema: Option<&str>) -> QueryResult<Vec<P::Row>>
    where
        P: ReadPragma<Scope = PerSchema>,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        self.all_pragma_rows::<P>(Some(schema.unwrap_or("main")))
    }

    /// Execute a per-schema pragma assignment (`None` selects `main`) without confirming the applied value.
    pub fn set_pragma<P>(&mut self, schema: Option<&str>, value: &P::Value) -> QueryResult<()>
    where
        P: WritePragma<Scope = PerSchema>,
    {
        PragmaQuery::<P>::write(Some(schema.unwrap_or("main")), value)
            .execute(self)
            .map(|_| ())
    }

    /// Read the first row of a per-connection pragma, returning [`Error::NotFound`] if empty.
    pub fn connection_pragma<P>(&mut self) -> QueryResult<P::Row>
    where
        P: ReadPragma<Scope = PerConnection>,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        self.first_pragma_row::<P>(None)
    }

    /// Read every row of a per-connection pragma.
    pub fn connection_pragma_rows<P>(&mut self) -> QueryResult<Vec<P::Row>>
    where
        P: ReadPragma<Scope = PerConnection>,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        self.all_pragma_rows::<P>(None)
    }

    /// Execute a per-connection pragma assignment without confirming the applied value.
    pub fn set_connection_pragma<P>(&mut self, value: &P::Value) -> QueryResult<()>
    where
        P: WritePragma<Scope = PerConnection>,
    {
        PragmaQuery::<P>::write(None, value)
            .execute(self)
            .map(|_| ())
    }

    fn pragma_cursor<'conn, 'query, P>(
        &'conn mut self,
        schema: Option<&'query str>,
    ) -> QueryResult<<Self as LoadConnection<DefaultLoadingMode>>::Cursor<'conn, 'query>>
    where
        P: ReadPragma,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        LoadConnection::<DefaultLoadingMode>::load(self, PragmaQuery::<P>::read(schema))
    }

    fn first_pragma_row<P>(&mut self, schema: Option<&str>) -> QueryResult<P::Row>
    where
        P: ReadPragma,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        let mut cursor = self.pragma_cursor::<P>(schema)?;
        let row = cursor.next().ok_or(Error::NotFound)??;
        decode_pragma_row::<P>(&row)
    }

    fn all_pragma_rows<P>(&mut self, schema: Option<&str>) -> QueryResult<Vec<P::Row>>
    where
        P: ReadPragma,
        Sqlite: QueryMetadata<P::SqlType>,
    {
        self.pragma_cursor::<P>(schema)?
            .map(|row| decode_pragma_row::<P>(&row?))
            .collect()
    }
}

fn decode_pragma_row<'a, P: ReadPragma>(row: &impl Row<'a, Sqlite>) -> QueryResult<P::Row> {
    P::Row::build_from_row(row).map_err(Error::DeserializationError)
}

struct PragmaQuery<'a, P> {
    schema: Option<&'a str>,
    value: Option<PragmaLiteral>,
    pragma: PhantomData<P>,
}

impl<'a, P: Pragma> PragmaQuery<'a, P> {
    fn read(schema: Option<&'a str>) -> Self {
        Self {
            schema,
            value: None,
            pragma: PhantomData,
        }
    }
}

impl<'a, P: WritePragma> PragmaQuery<'a, P> {
    fn write(schema: Option<&'a str>, value: &P::Value) -> Self {
        Self {
            schema,
            value: Some(value.to_pragma_literal()),
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
