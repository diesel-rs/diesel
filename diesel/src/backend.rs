//! Types which represent various database backends
use crate::query_builder::QueryBuilder;
use crate::sql_types::{self, HasSqlType, TypeMetadata};

/// A database backend
///
/// This trait represents the concept of a backend (e.g. "MySQL" vs "SQLite").
/// It is separate from a [`Connection`](crate::connection::Connection)
/// to that backend.
/// One backend may have multiple concrete connection implementations.
///
/// Implementations of this trait should not assume details about how the
/// connection is implemented.
/// For example, the `Pg` backend does not assume that `libpq` is being used.
/// Implementations of this trait can and should care about details of the wire
/// protocol used to communicated with the database.
pub trait Backend
where
    Self: Sized + SqlDialect,
    Self: HasSqlType<sql_types::SmallInt>,
    Self: HasSqlType<sql_types::Integer>,
    Self: HasSqlType<sql_types::BigInt>,
    Self: HasSqlType<sql_types::Float>,
    Self: HasSqlType<sql_types::Double>,
    Self: HasSqlType<sql_types::VarChar>,
    Self: HasSqlType<sql_types::Text>,
    Self: HasSqlType<sql_types::Binary>,
    Self: HasSqlType<sql_types::Date>,
    Self: HasSqlType<sql_types::Time>,
    Self: HasSqlType<sql_types::Timestamp>,
    Self: for<'a> HasRawValue<'a>,
    Self: for<'a> HasBindCollector<'a>,
{
    /// The concrete `QueryBuilder` implementation for this backend.
    type QueryBuilder: QueryBuilder<Self>;
}

/// The bind collector type used to collect query binds for this backend
///
/// This trait is separate from `Backend` to imitate `type BindCollector<'a>`. It
/// should only be referenced directly by implementors. Users of this type
/// should instead use the [`BindCollector`] helper type instead.
pub trait HasBindCollector<'a>: TypeMetadata + Sized {
    /// The concrete `BindCollector` implementation for this backend.
    ///
    /// Most backends should use [`RawBytesBindCollector`].
    ///
    /// [`RawBytesBindCollector`]: crate::query_builder::bind_collector::RawBytesBindCollector
    type BindCollector: crate::query_builder::bind_collector::BindCollector<'a, Self> + 'a;
}

/// The raw representation of a database value given to `FromSql`.
///
/// This trait is separate from `Backend` to imitate `type RawValue<'a>`. It
/// should only be referenced directly by implementors. Users of this type
/// should instead use the [`RawValue`] helper type instead.
pub trait HasRawValue<'a> {
    /// The actual type given to `FromSql`, with lifetimes applied. This type
    /// should not be used directly. Use the [`RawValue`]
    /// helper type instead.
    type RawValue;
}

/// A helper type to get the raw representation of a database type given to
/// `FromSql`. Equivalent to `<DB as Backend>::RawValue<'a>`.
pub type RawValue<'a, DB> = <DB as HasRawValue<'a>>::RawValue;

/// A helper type to get the bind collector for a database backend.
/// Equivalent to `<DB as HasBindCollector<'a>>::BindCollector<'a>`j
pub type BindCollector<'a, DB> = <DB as HasBindCollector<'a>>::BindCollector;

/// This trait provides various options to configure the
/// generated SQL for a specific backend.
///
/// Accessing anything from this trait is considered to be part of the
/// public API. Implementing this trait is not considered to be part of
/// diesels public API, as future versions of diesel may add additional
/// associated constants here.
///
/// Each associated type is used to configure the behaviour
/// of one or more [`QueryFragment`](crate::query_builder::QueryFragment)
/// implementations by providing
/// a custom `QueryFragment<YourBackend, YourSpecialSyntaxType>` implementation
/// to specialize on generic `QueryFragment<DB, DB::AssociatedType>` implementations.
///
/// See the [`sql_dialect`] module for options provided by diesel out of the box.
pub trait SqlDialect {
    /// Configures how this backends supports `RETURNING` clauses
    ///
    /// This allows backends to opt in  `RETURNING` clause support and to
    /// provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementation for [`ReturningClause`](crate::query_builder::ReturningClause)
    type ReturningClause;
    /// Configures how this backend supports `ON CONFLICT` clauses
    ///
    /// This allows backends to opt in `ON CONFLICT` clause support
    type OnConflictClause;
    /// Configures how this backend handles the bare `DEFAULT` keyword for
    /// inserting the default value in a `INSERT INTO` `VALUES` clause
    ///
    /// This allows backends to opt in support for `DEFAULT` value expressions
    /// for insert statements
    type InsertWithDefaultKeyword;
    /// Configures how this backend handles Batch insert statements
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementation for [`BatchInsert`](crate::query_builder::BatchInsert)
    type BatchInsertSupport;
    /// Configures how this backend handles the `DEFAULT VALUES` clause for
    /// insert statements.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementation for [`DefaultValues`](crate::query_builder::DefaultValues)
    type DefaultValueClauseForInsert;
    /// Configures how this backend handles empty `FROM` clauses for select statements.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementation for [`NoFromClause`](crate::query_builder::NoFromClause)
    type EmptyFromClauseSyntax;
    /// Configures how this backend handles `EXISTS()` expressions.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementation for [`Exists`](crate::expression::exists::Exists)
    type ExistsSyntax;

    /// Configures how this backend handles `IN()` and `NOT IN()` expressions.
    ///
    /// This allows backends to provide custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementations for [`In`](crate::expression::array_comparison::In),
    /// [`NotIn`](crate::expression::array_comparison::NotIn) and
    /// [`Many`](crate::expression::array_comparison::Many)
    type ArrayComparision;
}

/// This module contains all options provided by diesel to configure the [`SqlDialect`] trait.
pub mod sql_dialect {
    #[cfg(doc)]
    use super::SqlDialect;

    /// This module contains all diesel provided reusable options to
    /// configure [`SqlDialect::OnConflictClause`]
    pub mod on_conflict_clause {
        /// A marker trait indicating if a `ON CONFLICT` clause is supported or not
        ///
        /// If you use a custom type to specify specialized support for `ON CONFLICT` clauses
        /// implementing this trait opts into reusing diesels existing `ON CONFLICT`
        /// `QueryFragment` implementations
        pub trait SupportsOnConflictClause {}

        /// This marker type indicates that `ON CONFLICT` clauses are not supported for this backend
        #[derive(Debug, Copy, Clone)]
        pub struct DoesNotSupportOnConflictClause;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::ReturningClause`]
    pub mod returning_clause {
        /// A marker trait indicating if a `RETURNING` clause is supported or not
        ///
        /// If you use custom type to specify specialized support for `RETURNING` clauses
        /// implementing this trait opts in supporting `RETURNING` clause syntax
        pub trait SupportsReturningClause {}

        /// Indicates that a backend provides support for `RETURNING` clauses
        /// using the postgresql `RETURNING` syntax
        #[derive(Debug, Copy, Clone)]
        pub struct PgLikeReturningClause;

        /// Indicates that a backend does not support `RETURNING` clauses
        #[derive(Debug, Copy, Clone)]
        pub struct DoesNotSupportReturningClause;

        impl SupportsReturningClause for PgLikeReturningClause {}
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::InsertWithDefaultKeyword`]
    pub mod default_keyword_for_insert {
        /// A marker trait indicating if a `DEFAULT` like expression
        /// is supported as part of `INSERT INTO` clauses to indicate
        /// that a default value should be inserted at a specific position
        ///
        /// If you use a custom type to specify specialized support for `DEFAULT`
        /// expressions implementing this trait opts in support for `DEFAULT`
        /// value expressions for inserts. Otherwise diesel will emulate this
        /// behaviour.
        pub trait SupportsDefaultKeyword {}

        /// Indicates that a backend support `DEFAULT` value expressions
        /// for `INSERT INTO` statements based on the ISO SQL standard
        #[derive(Debug, Copy, Clone)]
        pub struct IsoSqlDefaultKeyword;

        /// Indicates that a backend does not support `DEFAULT` value
        /// expressions0for `INSERT INTO` statements
        #[derive(Debug, Copy, Clone)]
        pub struct DoesNotSupportDefaultKeyword;

        impl SupportsDefaultKeyword for IsoSqlDefaultKeyword {}
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::BatchInsertSupport`]
    pub mod batch_insert_support {
        /// A marker trait indicating if batch insert statements
        /// are supported for this backend or not
        pub trait SupportsBatchInsert {}

        /// Indicates that this backend does not support batch
        /// insert statements.
        /// In this case diesel will emulate batch insert support
        /// by inserting each row on it's own
        #[derive(Debug, Copy, Clone)]
        pub struct DoesNotSupportBatchInsert;

        /// Indicates that this backend supports postgres style
        /// batch insert statements to insert multiple rows using one
        /// insert statement
        #[derive(Debug, Copy, Clone)]
        pub struct PostgresLikeBatchInsertSupport;

        impl SupportsBatchInsert for PostgresLikeBatchInsertSupport {}
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::DefaultValueClauseForInsert`]
    pub mod default_value_clause {

        /// Indicates that this backend uses the
        /// `DEFAULT VALUES` syntax to specify
        /// that a row consisting only of default
        /// values should be inserted
        #[derive(Debug, Clone, Copy)]
        pub struct AnsiDefaultValueClause;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::EmptyFromClauseSyntax`]
    pub mod from_clause_syntax {

        /// Indicates that this backend skips
        /// the `FROM` clause in `SELECT` statements
        /// if no table/view is queried
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlFromClauseSyntax;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::ExistsSyntax`]
    pub mod exists_syntax {

        /// Indicates that this backend
        /// treats `EXIST()` as function
        /// like expression
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlExistsSyntax;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::ArrayComparision`]
    pub mod array_comparision {

        /// Indicates that this backend requires a single bind
        /// per array element in `IN()` and `NOT IN()` expression
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlArrayComparison;
    }
}
