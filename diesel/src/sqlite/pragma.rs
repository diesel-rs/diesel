//! Typed access to SQLite [pragmas](https://www.sqlite.org/pragma.html).
//!
//! A pragma is declared as a type implementing [`Pragma`], with [`ReadPragma`] when it returns rows and [`WritePragma`] when it can be assigned an argument. Its [`Scope`](Pragma::Scope) decides whether it is used per database or per connection, and the caller picks the Rust type each row decodes into.
//!
//! # Per-schema pragmas
//!
//! A [`PerSchema`] pragma applies to one database and is read and written through [`SchemaPragma`], where a `None` schema selects `main`.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::sqlite::pragma::{PerSchema, Pragma, ReadPragma, SchemaPragma, WritePragma};
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
//! impl WritePragma for UserVersion {
//!     type Argument<'a> = i32;
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! UserVersion::set(7, None, conn)?;
//! assert_eq!(7, UserVersion::get::<i32>(None, conn)?);
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
//! use diesel::sqlite::pragma::{
//!     ConnectionPragma, PerConnection, Pragma, ReadPragma, WritePragma,
//! };
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
//! impl WritePragma for ForeignKeys {
//!     type Argument<'a> = bool;
//! }
//!
//! # fn main() -> QueryResult<()> {
//! let conn = &mut SqliteConnection::establish(":memory:").unwrap();
//! ForeignKeys::set(true, conn)?;
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
use crate::sqlite::{AutoVacuumMode, Sqlite, SqliteConnection};
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
    use crate::query_builder::AstPass;
    use crate::result::QueryResult;
    use crate::sqlite::Sqlite;

    pub trait Sealed {}
    impl Sealed for super::PerSchema {}
    impl Sealed for super::PerConnection {}

    pub trait RenderArgument {
        fn render(&self, out: &mut AstPass<'_, '_, Sqlite>) -> QueryResult<()>;
    }
}

/// A pragma that returns rows when queried.
pub trait ReadPragma: Pragma {
    /// The SQL type of one returned row.
    type SqlType: SqlType;
}

/// A pragma that can be assigned an [`Argument`](Self::Argument).
///
/// Never assign secrets this way, since [`Instrumentation`](crate::connection::Instrumentation), [`on_trace`](SqliteConnection::on_trace) and [`on_authorize`](SqliteConnection::on_authorize) observe the statement.
pub trait WritePragma: Pragma {
    /// The argument assigned to the pragma, accepted as any type converting into it.
    type Argument<'a>: PragmaArgument;
}

/// A value written into a pragma statement, which takes no bind parameters.
///
/// Diesel renders every argument itself, so this trait cannot be implemented outside Diesel. A type of your own becomes an argument by converting into one with [`From`].
///
/// Integers and `bool` are written as decimal integers.
pub trait PragmaArgument: private::RenderArgument {}

/// A keyword written unquoted, such as `WAL` or `NORMAL`.
///
/// A keyword that is empty or holds anything but ASCII letters, digits and underscores fails with [`Error::QueryBuilderError`] before reaching SQLite.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel::sqlite::pragma::{
///     PerSchema, Pragma, PragmaKeyword, ReadPragma, SchemaPragma, WritePragma,
/// };
///
/// struct Synchronous;
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
///     type Argument<'a> = PragmaKeyword;
/// }
///
/// enum SynchronousMode {
///     Off,
///     Normal,
/// }
///
/// impl From<SynchronousMode> for PragmaKeyword {
///     fn from(mode: SynchronousMode) -> Self {
///         match mode {
///             SynchronousMode::Off => PragmaKeyword::new("OFF"),
///             SynchronousMode::Normal => PragmaKeyword::new("NORMAL"),
///         }
///     }
/// }
///
/// # fn main() -> QueryResult<()> {
/// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
/// Synchronous::set(SynchronousMode::Normal, None, conn)?;
/// assert_eq!(1, Synchronous::get::<i32>(None, conn)?);
/// Synchronous::set(SynchronousMode::Off, None, conn)?;
/// assert_eq!(0, Synchronous::get::<i32>(None, conn)?);
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PragmaKeyword(&'static str);

impl PragmaKeyword {
    /// Wrap `keyword`, which is checked when the statement is built.
    pub const fn new(keyword: &'static str) -> Self {
        Self(keyword)
    }
}

impl private::RenderArgument for PragmaKeyword {
    fn render(&self, out: &mut AstPass<'_, '_, Sqlite>) -> QueryResult<()> {
        push_bare_word(out, "keyword", self.0)
    }
}

impl PragmaArgument for PragmaKeyword {}

/// Text written as a single-quoted SQL string with embedded quotes doubled, such as `'UTF-16le'`.
///
/// Text holding a NUL byte fails with [`Error::InvalidCString`] before reaching SQLite.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel::sqlite::pragma::{
///     ConnectionPragma, PerConnection, Pragma, PragmaText, ReadPragma, WritePragma,
/// };
///
/// struct Encoding;
///
/// impl Pragma for Encoding {
///     const NAME: &'static str = "encoding";
///     type Scope = PerConnection;
/// }
///
/// impl ReadPragma for Encoding {
///     type SqlType = Text;
/// }
///
/// impl WritePragma for Encoding {
///     type Argument<'a> = PragmaText<'a>;
/// }
///
/// # // Reading text allocated by the native sqlite library is not supported
/// # // by miri `-Zmiri-native-lib`.
/// # #[cfg(not(miri))]
/// # fn main() -> QueryResult<()> {
/// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
/// // SQLite only changes the encoding before anything is written.
/// Encoding::set("UTF-16le", conn)?;
/// assert_eq!("UTF-16le", Encoding::get::<String>(conn)?);
/// #     Ok(())
/// # }
/// # #[cfg(miri)]
/// # fn main() {}
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PragmaText<'a>(&'a str);

impl<'a> From<&'a str> for PragmaText<'a> {
    fn from(text: &'a str) -> Self {
        Self(text)
    }
}

impl private::RenderArgument for PragmaText<'_> {
    fn render(&self, out: &mut AstPass<'_, '_, Sqlite>) -> QueryResult<()> {
        out.push_sql("'");
        for (index, fragment) in self.0.split('\'').enumerate() {
            if index != 0 {
                out.push_sql("''");
            }
            out.push_sql(fragment);
        }
        out.push_sql("'");
        Ok(())
    }
}

impl PragmaArgument for PragmaText<'_> {}

macro_rules! integer_pragma_argument {
    ($($ty:ty),*) => {$(
        impl private::RenderArgument for $ty {
            fn render(&self, out: &mut AstPass<'_, '_, Sqlite>) -> QueryResult<()> {
                out.push_sql(&i64::from(*self).to_string());
                Ok(())
            }
        }

        impl PragmaArgument for $ty {}
    )*};
}

integer_pragma_argument!(i8, i16, i32, i64, u8, u16, u32, bool);

/// Reads and writes a [`PerSchema`] pragma, where a `None` schema selects `main`.
pub trait SchemaPragma: Pragma<Scope = PerSchema> {
    /// Read the first row, returning [`Error::NotFound`] if the pragma returns none.
    fn get<Row>(schema: Option<&str>, conn: &mut SqliteConnection) -> QueryResult<Row>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
        Row: FromSqlRow<Self::SqlType, Sqlite>,
    {
        first_row::<Self, Row>(conn, Some(schema.unwrap_or("main")))
    }

    /// Read every row.
    fn get_all<Row>(schema: Option<&str>, conn: &mut SqliteConnection) -> QueryResult<Vec<Row>>
    where
        Self: ReadPragma,
        Sqlite: QueryMetadata<Self::SqlType>,
        Row: FromSqlRow<Self::SqlType, Sqlite>,
    {
        all_rows::<Self, Row>(conn, Some(schema.unwrap_or("main")))
    }

    /// Assign `argument`, running the statement to completion without confirming the value SQLite applied.
    fn set<'a>(
        argument: impl Into<Self::Argument<'a>>,
        schema: Option<&str>,
        conn: &mut SqliteConnection,
    ) -> QueryResult<()>
    where
        Self: WritePragma,
    {
        assign::<Self>(conn, Some(schema.unwrap_or("main")), argument.into())
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

    /// Assign `argument`, running the statement to completion without confirming the value SQLite applied.
    fn set<'a>(
        argument: impl Into<Self::Argument<'a>>,
        conn: &mut SqliteConnection,
    ) -> QueryResult<()>
    where
        Self: WritePragma,
    {
        assign::<Self>(conn, None, argument.into())
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
/// assert_eq!(0, PageCount::get::<i64>(None, conn)?);
/// conn.batch_execute("CREATE TABLE items (id INTEGER PRIMARY KEY)")?;
/// assert!(PageCount::get::<i64>(None, conn)? > 0);
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
/// assert_eq!(0, FreelistCount::get::<i64>(None, conn)?);
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

/// `PRAGMA auto_vacuum`, read and set as an [`AutoVacuumMode`].
///
/// Changing from or to [`AutoVacuumMode::None`] only takes effect on a database with no tables yet, or after a subsequent `VACUUM`.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel::sqlite::AutoVacuumMode;
/// use diesel::sqlite::pragma::{AutoVacuum, SchemaPragma};
///
/// # fn main() -> QueryResult<()> {
/// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
/// assert_eq!(AutoVacuumMode::None, AutoVacuum::get::<AutoVacuumMode>(None, conn)?);
/// AutoVacuum::set(AutoVacuumMode::Incremental, None, conn)?;
/// assert_eq!(
///     AutoVacuumMode::Incremental,
///     AutoVacuum::get::<AutoVacuumMode>(None, conn)?
/// );
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AutoVacuum;

impl Pragma for AutoVacuum {
    const NAME: &'static str = "auto_vacuum";
    type Scope = PerSchema;
}

impl ReadPragma for AutoVacuum {
    type SqlType = crate::sql_types::Integer;
}

impl WritePragma for AutoVacuum {
    type Argument<'a> = AutoVacuumMode;
}

impl private::RenderArgument for AutoVacuumMode {
    fn render(&self, out: &mut AstPass<'_, '_, Sqlite>) -> QueryResult<()> {
        // Discriminant extraction, `#[repr(i32)]` makes it the mode's SQLite number.
        out.push_sql(&(*self as i32).to_string());
        Ok(())
    }
}

impl PragmaArgument for AutoVacuumMode {}

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

fn assign<P: WritePragma>(
    conn: &mut SqliteConnection,
    schema: Option<&str>,
    argument: P::Argument<'_>,
) -> QueryResult<()> {
    use crate::connection::SimpleConnection;
    use crate::query_builder::QueryBuilder;
    use crate::sqlite::SqliteQueryBuilder;

    // Some pragmas do one unit of work per step, `incremental_vacuum` frees one page per step.
    // `sqlite3_exec` behind `batch_execute` runs to completion, while `StatementUse::run` steps once.
    let mut query = SqliteQueryBuilder::new();
    PragmaAssignment::<P, _> {
        schema,
        argument,
        pragma: PhantomData,
    }
    .to_sql(&mut query, &Sqlite)?;
    conn.batch_execute(&query.finish())
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
        push_pragma::<P>(&mut out, self.schema)
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

struct PragmaAssignment<'a, P, A> {
    schema: Option<&'a str>,
    argument: A,
    pragma: PhantomData<P>,
}

impl<P: Pragma, A: PragmaArgument> QueryFragment<Sqlite> for PragmaAssignment<'_, P, A> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        push_pragma::<P>(&mut out, self.schema)?;
        out.push_sql(" = ");
        self.argument.render(&mut out)
    }
}

fn push_pragma<P: Pragma>(
    out: &mut AstPass<'_, '_, Sqlite>,
    schema: Option<&str>,
) -> QueryResult<()> {
    out.push_sql("PRAGMA ");
    if let Some(schema) = schema {
        out.push_identifier(schema)?;
        out.push_sql(".");
    }
    push_bare_word(out, "name", P::NAME)
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
