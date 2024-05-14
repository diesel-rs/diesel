//! Types which represent various database backends

use crate::query_builder::QueryBuilder;
use crate::sql_types::{self, HasSqlType, TypeMetadata};

#[cfg_attr(
    not(any(
        feature = "postgres_backend",
        feature = "mysql_backend",
        feature = "sqlite",
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )),
    allow(unused_imports)
)]
#[doc(inline)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::private::{DieselReserveSpecialization, TrustedBackend};

/// A database backend
///
/// This trait represents the concept of a backend (e.g. "MySQL" vs "SQLite").
/// It is separate from a [`Connection`](crate::connection::Connection)
/// to that backend.
/// One backend may have multiple concrete connection implementations.
///
/// # Implementing a custom backend
///
/// Implementing a custom backend requires enabling the
/// `i-implement-a-third-party-backend-and-opt-into-breaking-changes` crate feature
/// to get access to all necessary type and trait implementations.
///
/// Implementations of this trait should not assume details about how the
/// connection is implemented.
/// For example, the `Pg` backend does not assume that `libpq` is being used.
/// Implementations of this trait can and should care about details of the wire
/// protocol used to communicate with the database.
///
/// Implementing support for a new backend is a complex topic and depends on the
/// details how the newly implemented backend may communicate with diesel. As of this,
/// we cannot provide concrete examples here and only present a general outline of
/// the required steps. Existing backend implementations provide a good starting point
/// to see how certain things are solved for other backend implementations.
///
/// Types implementing `Backend` should generally be zero sized structs.
///
/// To implement the `Backend` trait, you need to:
///
/// * Specify how a query should be build from string parts by providing a [`QueryBuilder`]
/// matching your backend
/// * Specify the bind value format used by your database connection library by providing
/// a [`BindCollector`](crate::query_builder::bind_collector::BindCollector) matching your backend
/// * Specify how values are received from the database by providing a corresponding raw value
/// definition
/// * Control sql dialect specific parts of diesels query dsl implementation by providing a
/// matching [`SqlDialect`] implementation
/// * Implement [`TypeMetadata`] to specify how your backend identifies types
/// * Specify support for common datatypes by implementing [`HasSqlType`] for the following sql types:
///     + [`SmallInt`](sql_types::SmallInt)
///     + [`Integer`](sql_types::Integer)
///     + [`BigInt`](sql_types::BigInt)
///     + [`Float`](sql_types::Float)
///     + [`Double`](sql_types::Double)
///     + [`Text`](sql_types::Text)
///     + [`Binary`](sql_types::Binary)
///     + [`Date`](sql_types::Date)
///     + [`Time`](sql_types::Time)
///     + [`Timestamp`](sql_types::Timestamp)
///
/// Additionally to the listed required trait bounds you may want to implement
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    doc = "[`DieselReserveSpecialization`]"
)]
#[cfg_attr(
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
    doc = "`DieselReserveSpecialization`"
)]
/// to opt in existing wild card [`QueryFragment`] impls for large parts of the dsl.
///
/// [`QueryFragment`]: crate::query_builder::QueryFragment
pub trait Backend
where
    Self: Sized + SqlDialect + TypeMetadata,
    Self: HasSqlType<sql_types::SmallInt>,
    Self: HasSqlType<sql_types::Integer>,
    Self: HasSqlType<sql_types::BigInt>,
    Self: HasSqlType<sql_types::Float>,
    Self: HasSqlType<sql_types::Double>,
    Self: HasSqlType<sql_types::Text>,
    Self: HasSqlType<sql_types::Binary>,
    Self: HasSqlType<sql_types::Date>,
    Self: HasSqlType<sql_types::Time>,
    Self: HasSqlType<sql_types::Timestamp>,
{
    /// The concrete [`QueryBuilder`] implementation for this backend.
    type QueryBuilder: QueryBuilder<Self>;

    /// The actual type given to [`FromSql`], with lifetimes applied. This type
    /// should not be used directly.
    ///
    /// [`FromSql`]: crate::deserialize::FromSql
    type RawValue<'a>;

    /// The concrete [`BindCollector`](crate::query_builder::bind_collector::BindCollector)
    /// implementation for this backend.
    ///
    /// Most backends should use [`RawBytesBindCollector`].
    ///
    /// [`RawBytesBindCollector`]: crate::query_builder::bind_collector::RawBytesBindCollector
    type BindCollector<'a>: crate::query_builder::bind_collector::BindCollector<'a, Self> + 'a;
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(note = "Use `Backend::RawValue` directly")]
pub type RawValue<'a, DB> = <DB as Backend>::RawValue<'a>;

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(note = "Use `Backend::BindCollector` directly")]
pub type BindCollector<'a, DB> = <DB as Backend>::BindCollector<'a>;

/// This trait provides various options to configure the
/// generated SQL for a specific backend.
///
/// Accessing anything from this trait is considered to be part of the
/// public API. Implementing this trait is not considered to be part of
/// diesel's public API, as future versions of diesel may add additional
/// associated constants here.
///
/// Each associated type is used to configure the behaviour
/// of one or more [`QueryFragment`](crate::query_builder::QueryFragment)
/// implementations by providing
/// a custom `QueryFragment<YourBackend, YourSpecialSyntaxType>` implementation
/// to specialize on generic `QueryFragment<DB, DB::AssociatedType>` implementations.
///
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    doc = "See the [`sql_dialect`] module for options provided by diesel out of the box."
)]
pub trait SqlDialect: self::private::TrustedBackend {
    /// Configures how this backend supports `RETURNING` clauses
    ///
    /// This allows backends to opt in `RETURNING` clause support and to
    /// provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementation for [`ReturningClause`](crate::query_builder::ReturningClause)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementation for `ReturningClause`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::returning_clause`] for provided default implementations"
    )]
    type ReturningClause;
    /// Configures how this backend supports `ON CONFLICT` clauses
    ///
    /// This allows backends to opt in `ON CONFLICT` clause support
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::on_conflict_clause`] for provided default implementations"
    )]
    type OnConflictClause;
    /// Configures how this backend handles the bare `DEFAULT` keyword for
    /// inserting the default value in a `INSERT INTO` `VALUES` clause
    ///
    /// This allows backends to opt in support for `DEFAULT` value expressions
    /// for insert statements
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::default_keyword_for_insert`] for provided default implementations"
    )]
    type InsertWithDefaultKeyword;
    /// Configures how this backend handles Batch insert statements
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementation for [`BatchInsert`](crate::query_builder::BatchInsert)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementation for `BatchInsert`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::batch_insert_support`] for provided default implementations"
    )]
    type BatchInsertSupport;
    /// Configures how this backend handles the Concat clauses in
    /// select statements.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementation for [`Concat`](crate::expression::Concat)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementation for `Concat`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::concat_clause`] for provided default implementations"
    )]
    type ConcatClause;
    /// Configures how this backend handles the `DEFAULT VALUES` clause for
    /// insert statements.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementation for [`DefaultValues`](crate::query_builder::DefaultValues)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementation for `DefaultValues`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::default_value_clause`] for provided default implementations"
    )]
    type DefaultValueClauseForInsert;
    /// Configures how this backend handles empty `FROM` clauses for select statements.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementation for [`NoFromClause`](crate::query_builder::NoFromClause)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementation for `NoFromClause`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::from_clause_syntax`] for provided default implementations"
    )]
    type EmptyFromClauseSyntax;
    /// Configures how this backend handles `EXISTS()` expressions.
    ///
    /// This allows backends to provide a custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementation for [`Exists`](crate::expression::exists::Exists)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementation for `Exists`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::exists_syntax`] for provided default implementations"
    )]
    type ExistsSyntax;

    /// Configures how this backend handles `IN()` and `NOT IN()` expressions.
    ///
    /// This allows backends to provide custom [`QueryFragment`](crate::query_builder::QueryFragment)
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "implementations for [`In`](crate::expression::array_comparison::In),
    [`NotIn`](crate::expression::array_comparison::NotIn) and
    [`Many`](crate::expression::array_comparison::Many)"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "implementations for `In`, `NotIn` and `Many`"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::array_comparison`] for provided default implementations"
    )]
    type ArrayComparison;

    /// Configures how this backend structures `SELECT` queries
    ///
    /// This allows backends to provide custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementations for
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "`SelectStatement` and `BoxedSelectStatement`"
    )]
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc = "[`SelectStatement`](crate::query_builder::SelectStatement) and
               [`BoxedSelectStatement`](crate::query_builder::BoxedSelectStatement)"
    )]
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::select_statement_syntax`] for provided default implementations"
    )]
    type SelectStatementSyntax;

    /// Configures how this backend structures `SELECT` queries
    ///
    /// This allows backends to provide custom [`QueryFragment`](crate::query_builder::QueryFragment)
    /// implementations for [`Alias<T>`](crate::query_source::Alias)
    ///
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        doc = "See [`sql_dialect::alias_syntax`] for provided default implementations"
    )]
    type AliasSyntax;
}

/// This module contains all options provided by diesel to configure the [`SqlDialect`] trait.
// This module is only public behind the unstable feature flag, as we may want to change SqlDialect
// implementations of existing backends because of:
// * The backend gained support for previously unsupported SQL operations
// * The backend fixed/introduced a bug that requires special handling
// * We got some edge case wrong with sharing the implementation between backends
//
// By not exposing these types publicly we are able to change the exact definitions later on
// as users cannot write trait bounds that ensure that a specific type is used in place of
// an existing associated type.
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) mod sql_dialect {
    #![cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        // Otherwise there are false positives
        // because the lint seems to believe that these pub statements
        // are not required, but they are required through the various backend impls
        allow(unreachable_pub)
    )]
    #[cfg(doc)]
    use super::SqlDialect;

    /// This module contains all diesel provided reusable options to
    /// configure [`SqlDialect::OnConflictClause`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod on_conflict_clause {
        /// A marker trait indicating if a `ON CONFLICT` clause is supported or not
        ///
        /// If you use a custom type to specify specialized support for `ON CONFLICT` clauses
        /// implementing this trait opts into reusing diesels existing `ON CONFLICT`
        /// `QueryFragment` implementations
        pub trait SupportsOnConflictClause {}

        /// A marker trait indicating if a `ON CONFLICT (...) DO UPDATE ... [WHERE ...]` clause is supported or not
        pub trait SupportsOnConflictClauseWhere {}

        /// A marker trait indicating whether the on conflict clause implementation
        /// is mostly like postgresql
        pub trait PgLikeOnConflictClause: SupportsOnConflictClause {}

        /// This marker type indicates that `ON CONFLICT` clauses are not supported for this backend
        #[derive(Debug, Copy, Clone)]
        pub struct DoesNotSupportOnConflictClause;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::ReturningClause`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod returning_clause {
        /// A marker trait indicating if a `RETURNING` clause is supported or not
        ///
        /// If you use a custom type to specify specialized support for `RETURNING` clauses
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
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
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
        /// expressions for `INSERT INTO` statements
        #[derive(Debug, Copy, Clone)]
        pub struct DoesNotSupportDefaultKeyword;

        impl SupportsDefaultKeyword for IsoSqlDefaultKeyword {}
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::BatchInsertSupport`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod batch_insert_support {
        /// A marker trait indicating if batch insert statements
        /// are supported for this backend or not
        pub trait SupportsBatchInsert {}

        /// Indicates that this backend does not support batch
        /// insert statements.
        /// In this case diesel will emulate batch insert support
        /// by inserting each row on its own
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
    /// [`SqlDialect::ConcatClause`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod concat_clause {

        /// Indicates that this backend uses the
        /// `||` operator to select a concatenation
        /// of two variables or strings
        #[derive(Debug, Clone, Copy)]
        pub struct ConcatWithPipesClause;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::DefaultValueClauseForInsert`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
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
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub(crate) mod from_clause_syntax {

        /// Indicates that this backend skips
        /// the `FROM` clause in `SELECT` statements
        /// if no table/view is queried
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlFromClauseSyntax;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::ExistsSyntax`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod exists_syntax {

        /// Indicates that this backend
        /// treats `EXIST()` as function
        /// like expression
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlExistsSyntax;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::ArrayComparison`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod array_comparison {

        /// Indicates that this backend requires a single bind
        /// per array element in `IN()` and `NOT IN()` expression
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlArrayComparison;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::SelectStatementSyntax`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod select_statement_syntax {
        /// Indicates that this backend uses the default
        /// ANSI select statement structure
        #[derive(Debug, Copy, Clone)]
        pub struct AnsiSqlSelectStatement;
    }

    /// This module contains all reusable options to configure
    /// [`SqlDialect::AliasSyntax`]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub mod alias_syntax {
        /// Indicates that this backend uses `table AS alias` for
        /// defining table aliases
        #[derive(Debug, Copy, Clone)]
        pub struct AsAliasSyntax;
    }
}

// These traits are not part of the public API
// because we want to replace them by with an associated type
// in the child trait later if GAT's are finally stable
pub(crate) mod private {

    /// This is a marker trait which indicates that
    /// diesel may specialize a certain [`QueryFragment`]
    /// impl in a later version. If you as a user encounter, where rustc
    /// suggests adding this a bound to a type implementing `Backend`
    /// consider adding the following bound instead
    /// `YourQueryType: QueryFragment<DB>` (the concrete bound
    /// is likely mentioned by rustc as part of a `note: …`)
    ///
    /// For any user implementing a custom backend: You likely want to implement
    /// this trait for your custom backend type to opt in the existing [`QueryFragment`] impls in diesel.
    /// As indicated by the `i-implement-a-third-party-backend-and-opt-into-breaking-changes` feature
    /// diesel reserves the right to specialize any generic [`QueryFragment`](crate::query_builder::QueryFragment)
    /// impl via [`SqlDialect`](super::SqlDialect) in a later minor version release
    ///
    /// [`QueryFragment`]: crate::query_builder::QueryFragment
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub trait DieselReserveSpecialization {}

    /// This trait just indicates that none implements
    /// [`SqlDialect`](super::SqlDialect) without enabling the
    /// `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
    /// feature flag.
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub trait TrustedBackend {}
}
