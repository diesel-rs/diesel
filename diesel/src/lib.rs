//! # Diesel
//!
//! Diesel is an ORM and query builder designed to reduce the boilerplate for database interactions.
//! If this is your first time reading this documentation,
//! we recommend you start with the [getting started guide].
//! We also have [many other long form guides].
//!
//! [getting started guide]: https://diesel.rs/guides/getting-started/
//! [many other long form guides]: https://diesel.rs/guides
//!
//! # Where to find things
//!
//! ## Declaring your schema
//!
//! For Diesel to validate your queries at compile time
//! it requires you to specify your schema in your code,
//! which you can do with [the `table!` macro][`table!`].
//! `diesel print-schema` can be used
//! to automatically generate these macro calls
//! (by connecting to your database and querying its schema).
//!
//!
//! ## Getting started
//!
//! Queries usually start from either a table, or a function like [`update`].
//! Those functions can be found [here](#functions).
//!
//! Diesel provides a [`prelude` module](prelude),
//! which exports most of the typically used traits and types.
//! We are conservative about what goes in this module,
//! and avoid anything which has a generic name.
//! Files which use Diesel are expected to have `use diesel::prelude::*;`.
//!
//! [`update`]: update()
//!
//! ## Constructing a query
//!
//! The tools the query builder gives you can be put into these three categories:
//!
//! - "Query builder methods" are things that map to portions of a whole query
//!   (such as `ORDER` and `WHERE`). These methods usually have the same name
//!   as the SQL they map to, except for `WHERE` which is called `filter` in Diesel
//!   (To not conflict with the Rust keyword).
//!   These methods live in [the `query_dsl` module](query_dsl).
//! - "Expression methods" are things you would call on columns
//!   or other individual values.
//!   These methods live in [the `expression_methods` module](expression_methods)
//!   You can often find these by thinking "what would this be called"
//!   if it were a method
//!   and typing that into the search bar
//!   (e.g. `LIKE` is called `like` in Diesel).
//!   Most operators are named based on the Rust function which maps to that
//!   operator in [`std::ops`][]
//!   (For example `==` is called `.eq`, and `!=` is called `.ne`).
//! - "Bare functions" are normal SQL functions
//!   such as `sum`.
//!   They live in [the `dsl` module](dsl).
//!   Diesel only supports a very small number of these functions.
//!   You can declare additional functions you want to use
//!   with [the `sql_function!` macro][`sql_function!`].
//!
//! [`std::ops`]: //doc.rust-lang.org/stable/std/ops/index.html
//!
//! ## Serializing and Deserializing
//!
//! Types which represent the result of a SQL query implement
//! a trait called [`Queryable`].
//!
//! Diesel maps "Rust types" (e.g. `i32`) to and from "SQL types"
//! (e.g. [`diesel::sql_types::Integer`]).
//! You can find all the types supported by Diesel in [the `sql_types` module](sql_types).
//! These types are only used to represent a SQL type.
//! You should never put them on your `Queryable` structs.
//!
//! To find all the Rust types which can be used with a given SQL type,
//! see the documentation for that SQL type.
//!
//! To find all the SQL types which can be used with a Rust type,
//! go to the docs for either [`ToSql`] or [`FromSql`],
//! go to the "Implementors" section,
//! and find the Rust type you want to use.
//!
//! [`Queryable`]: deserialize::Queryable
//! [`diesel::sql_types::Integer`]: sql_types::Integer
//! [`ToSql`]: serialize::ToSql
//! [`FromSql`]: deserialize::FromSql
//!
//! ## Getting help
//!
//! If you run into problems, Diesel has an active community.
//! Either open a new [discussion] thread at diesel github repository or
//! use the active Gitter room at
//! [gitter.im/diesel-rs/diesel](https://gitter.im/diesel-rs/diesel)
//!
//! [discussion]: https://github.com/diesel-rs/diesel/discussions/categories/q-a
//!
//! # Crate feature flags
//!
//! The following feature flags are considered to be part of diesels public
//! API. Any feature flag that is not listed here is **not** considered to
//! be part of the public API and can disappear at any point in time:

//!
//! - `sqlite`: This feature enables the diesel sqlite backend. Enabling this feature requires a compatible copy
//! of `libsqlite3` for your target architecture.
//! - `postgres`: This feature enables the diesel postgres backend. Enabling this feature requires a compatible
//! copy of `libpq` for your target architecture. This features implies `postgres_backend`
//! - `mysql`: This feature enables the idesel mysql backend. Enabling this feature requires a compatible copy
//! of `libmysqlclient` for your target architecture. This feature implies `mysql_backend`
//! - `postgres_backend`: This feature enables those parts of diesels postgres backend, that are not dependend
//! on `libpq`. Diesel does not provide any connection implementation with only this feature enabled.
//! This feature can be used to implement a custom implementation of diesels `Connection` trait for the
//! postgres backend outside of diesel itself, while reusing the existing query dsl extensions for the
//! postgres backend
//! - `mysql_backend`: This feature enables those parts of diesels mysql backend, that are not dependend
//! on `libmysqlclient`. Diesel does not provide any connection implementation with only this feature enabled.
//! This feature can be used to implement a custom implementation of diesels `Connection` trait for the
//! mysql backend outside of diesel itself, while reusing the existing query dsl extensions for the
//! mysql backend
//! - `returning_clauses_for_sqlite_3_35`: This feature enables support for `RETURNING` clauses in the sqlite backend.
//! Enabling this feature requires sqlite 3.35.0 or newer.
//! - `32-column-tables`: This feature enables support for tables with up to 32 columns.
//! This feature is enabled by default. Consider disabling this feature if you write a library crate
//! providing general extensions for diesel or if you do not need to support tables with more than 16 columns
//! and you want to minimize your compile times.
//! - `64-column-tables`: This feature enables support for tables with up to 64 columns. It implies the
//! `32-column-tables` feature. Enabling this feature will increase your compile times.
//! - `128-column-tables`: This feature enables support for tables with up to 128 columns. It implies the
//! `64-column-tables` feature. Enabling this feature will increase your compile times significantly.
//! - `i-implement-a-third-party-backend-and-opt-into-breaking-changes`: This feature opens up some otherwise
//! private API, that can be useful to implement a third party [`Backend`](crate::backend::Backend)
//! or write a custom [`Connection`](crate::connection::Connection) implementation. **Do not use this feature for
//! any other usecase**. By enabling this feature you explicitly opt out diesel stability gurantees. We explicitly
//! reserve us the right to break API's exported under this feature flag in any upcomming minor version release.
//! If you publish a crate depending on this feature flag consider to restrict the supported diesel version to the
//! currently released minor version.
//! - `serde_json`: This feature flag enables support for (de)serializing json values from the database using
//! types provided by `serde_json`.
//! - `chrono`: This feature flags enables support for (de)serializing date/time values from the database using
//! types provided by `chrono`
//! - `uuid`: This feature flag enables support for (de)serializing uuid values from the database using types
//! provided by `uuid`
//! - `network-address`: This feature flag enables support for (de)serializing
//! IP values from the database using types provided by `ipnetwork`.
//! - `ipnet-address`: This feature flag enables support for (de)serializing IP
//! values from the database using types provided by `ipnet`.
//! - `numeric`: This feature flag enables support for (de)serializing numeric values from the database using types
//! provided by `bigdecimal`
//! - `r2d2`: This feature flag enables support for the `r2d2` connection pool implementation.
//! - `extras`: This feature enables the feature flaged support for any third party crate. This implies the
//! following feature flags: `serde_json`, `chrono`, `uuid`, `network-address`, `numeric`, `r2d2`
//! - `with-deprecated`: This feature enables items marked as `#[deprecated]`. It is enabled by default.
//! disabling this feature explicitly opts out diesels stability gurantee.
//! - `without-deprecated`: This feature disables any item marked as `#[deprecated]`. Enabling this feature
//! explicitly opts out the stability gurantee given by diesel. This feature overrides the `with-deprecated`.
//! Note that this may also remove items that are not shown as `#[deprecated]` in our documentation, due to
//! various bugs in rustdoc. It can be used to check if you depend on any such hidden `#[deprecated]` item.
//!
//! By default the following features are enabled:
//!
//! - `with-deprecated`
//! - `32-column-tables`

#![cfg_attr(feature = "unstable", feature(trait_alias))]
#![cfg_attr(doc_cfg, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(feature = "128-column-tables", recursion_limit = "256")]
// Built-in Lints
#![warn(
    unreachable_pub,
    missing_debug_implementations,
    missing_copy_implementations,
    elided_lifetimes_in_paths,
    missing_docs
)]
// Clippy lints
#![allow(
    clippy::match_same_arms,
    clippy::needless_doctest_main,
    clippy::map_unwrap_or,
    clippy::redundant_field_names,
    clippy::type_complexity
)]
#![cfg_attr(test, allow(clippy::option_map_unwrap_or, clippy::result_unwrap_used))]
#![warn(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::enum_glob_use,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]

extern crate diesel_derives;

#[macro_use]
#[doc(hidden)]
pub mod macros;
#[doc(hidden)]
pub mod internal;

#[cfg(test)]
#[macro_use]
extern crate cfg_if;

#[cfg(test)]
pub mod test_helpers;

pub mod associations;
pub mod backend;
pub mod connection;
pub mod data_types;
pub mod deserialize;
#[macro_use]
pub mod expression;
pub mod expression_methods;
#[doc(hidden)]
pub mod insertable;
pub mod query_builder;
pub mod query_dsl;
pub mod query_source;
#[cfg(feature = "r2d2")]
pub mod r2d2;
pub mod result;
pub mod serialize;
pub mod upsert;
#[macro_use]
pub mod sql_types;
pub mod migration;
pub mod row;

#[cfg(feature = "mysql_backend")]
pub mod mysql;
#[cfg(feature = "postgres_backend")]
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod sqlite;

mod type_impls;
mod util;

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(since = "2.0.0", note = "Use explicit macro imports instead")]
pub use diesel_derives::{
    AsChangeset, AsExpression, Associations, DieselNumericOps, FromSqlRow, Identifiable,
    Insertable, QueryId, Queryable, QueryableByName, SqlType,
};

pub mod dsl {
    //! Includes various helper types and bare functions which are named too
    //! generically to be included in prelude, but are often used when using Diesel.

    #[doc(inline)]
    pub use crate::helper_types::*;

    #[doc(inline)]
    pub use crate::expression::dsl::*;

    #[doc(inline)]
    pub use crate::query_builder::functions::{
        delete, insert_into, insert_or_ignore_into, replace_into, select, sql_query, update,
    };
}

pub mod helper_types {
    //! Provide helper types for concisely writing the return type of functions.
    //! As with iterators, it is unfortunately difficult to return a partially
    //! constructed query without exposing the exact implementation of the
    //! function. Without higher kinded types, these various DSLs can't be
    //! combined into a single trait for boxing purposes.
    //!
    //! All types here are in the form `<FirstType as
    //! DslName<OtherTypes>>::Output`. So the return type of
    //! `users.filter(first_name.eq("John")).order(last_name.asc()).limit(10)` would
    //! be `Limit<Order<FindBy<users, first_name, &str>, Asc<last_name>>>`
    use super::query_builder::combination_clause::{self, CombinationClause};
    use super::query_builder::{locking_clause as lock, AsQuery};
    use super::query_dsl::methods::*;
    use super::query_dsl::*;
    use super::query_source::{aliasing, joins};
    use crate::query_builder::select_clause::SelectClause;

    #[doc(inline)]
    pub use crate::expression::helper_types::*;

    /// Represents the return type of [`.select(selection)`](crate::prelude::QueryDsl::select)
    pub type Select<Source, Selection> = <Source as SelectDsl<Selection>>::Output;

    /// Represents the return type of [`diesel::select(selection)`](crate::select)
    pub type BareSelect<Selection> = crate::query_builder::SelectStatement<
        crate::query_builder::NoFromClause,
        SelectClause<Selection>,
    >;

    /// Represents the return type of [`.filter(predicate)`](crate::prelude::QueryDsl::filter)
    pub type Filter<Source, Predicate> = <Source as FilterDsl<Predicate>>::Output;

    /// Represents the return type of [`.filter(lhs.eq(rhs))`](crate::prelude::QueryDsl::filter)
    pub type FindBy<Source, Column, Value> = Filter<Source, Eq<Column, Value>>;

    /// Represents the return type of [`.for_update()`](crate::prelude::QueryDsl::for_update)
    pub type ForUpdate<Source> = <Source as LockingDsl<lock::ForUpdate>>::Output;

    /// Represents the return type of [`.for_no_key_update()`](crate::prelude::QueryDsl::for_no_key_update)
    pub type ForNoKeyUpdate<Source> = <Source as LockingDsl<lock::ForNoKeyUpdate>>::Output;

    /// Represents the return type of [`.for_share()`](crate::prelude::QueryDsl::for_share)
    pub type ForShare<Source> = <Source as LockingDsl<lock::ForShare>>::Output;

    /// Represents the return type of [`.for_key_share()`](crate::prelude::QueryDsl::for_key_share)
    pub type ForKeyShare<Source> = <Source as LockingDsl<lock::ForKeyShare>>::Output;

    /// Represents the return type of [`.skip_locked()`](crate::prelude::QueryDsl::skip_locked)
    pub type SkipLocked<Source> = <Source as ModifyLockDsl<lock::SkipLocked>>::Output;

    /// Represents the return type of [`.no_wait()`](crate::prelude::QueryDsl::no_wait)
    pub type NoWait<Source> = <Source as ModifyLockDsl<lock::NoWait>>::Output;

    /// Represents the return type of [`.find(pk)`](crate::prelude::QueryDsl::find)
    pub type Find<Source, PK> = <Source as FindDsl<PK>>::Output;

    /// Represents the return type of [`.or_filter(predicate)`](crate::prelude::QueryDsl::or_filter)
    pub type OrFilter<Source, Predicate> = <Source as OrFilterDsl<Predicate>>::Output;

    /// Represents the return type of [`.order(ordering)`](crate::prelude::QueryDsl::order)
    pub type Order<Source, Ordering> = <Source as OrderDsl<Ordering>>::Output;

    /// Represents the return type of [`.then_order_by(ordering)`](crate::prelude::QueryDsl::then_order_by)
    pub type ThenOrderBy<Source, Ordering> = <Source as ThenOrderDsl<Ordering>>::Output;

    /// Represents the return type of [`.limit()`](crate::prelude::QueryDsl::limit)
    pub type Limit<Source> = <Source as LimitDsl>::Output;

    /// Represents the return type of [`.offset()`](crate::prelude::QueryDsl::offset)
    pub type Offset<Source> = <Source as OffsetDsl>::Output;

    /// Represents the return type of [`.inner_join(rhs)`](crate::prelude::QueryDsl::inner_join)
    pub type InnerJoin<Source, Rhs> =
        <Source as JoinWithImplicitOnClause<Rhs, joins::Inner>>::Output;

    /// Represents the return type of [`.inner_join(rhs.on(on))`](crate::prelude::QueryDsl::inner_join)
    pub type InnerJoinOn<Source, Rhs, On> =
        <Source as InternalJoinDsl<Rhs, joins::Inner, On>>::Output;

    /// Represents the return type of [`.left_join(rhs)`](crate::prelude::QueryDsl::left_join)
    pub type LeftJoin<Source, Rhs> =
        <Source as JoinWithImplicitOnClause<Rhs, joins::LeftOuter>>::Output;

    /// Represents the return type of [`.left_join(rhs.on(on))`](crate::prelude::QueryDsl::left_join)
    pub type LeftJoinOn<Source, Rhs, On> =
        <Source as InternalJoinDsl<Rhs, joins::LeftOuter, On>>::Output;

    /// Represents the return type of [`rhs.on(on)`](crate::query_dsl::JoinOnDsl::on)
    pub type On<Source, On> = joins::OnClauseWrapper<Source, On>;

    use super::associations::HasTable;
    use super::query_builder::{AsChangeset, IntoUpdateTarget, UpdateStatement};

    /// Represents the return type of [`update(lhs).set(rhs)`](crate::query_builder::UpdateStatement::set)
    pub type Update<Target, Changes> = UpdateStatement<
        <Target as HasTable>::Table,
        <Target as IntoUpdateTarget>::WhereClause,
        <Changes as AsChangeset>::Changeset,
    >;

    /// Represents the return type of [`.into_boxed::<'a, DB>()`](crate::prelude::QueryDsl::into_boxed)
    pub type IntoBoxed<'a, Source, DB> = <Source as BoxedDsl<'a, DB>>::Output;

    /// Represents the return type of [`.distinct()`](crate::prelude::QueryDsl::distinct)
    pub type Distinct<Source> = <Source as DistinctDsl>::Output;

    /// Represents the return type of [`.distinct_on(expr)`](crate::prelude::QueryDsl::distinct_on)
    #[cfg(feature = "postgres_backend")]
    pub type DistinctOn<Source, Expr> = <Source as DistinctOnDsl<Expr>>::Output;

    /// Represents the return type of [`.single_value()`](SingleValueDsl::single_value)
    pub type SingleValue<Source> = <Source as SingleValueDsl>::Output;

    /// Represents the return type of [`.nullable()`](SelectNullableDsl::nullable)
    pub type NullableSelect<Source> = <Source as SelectNullableDsl>::Output;

    /// Represents the return type of [`.group_by(expr)`](crate::prelude::QueryDsl::group_by)
    pub type GroupBy<Source, Expr> = <Source as GroupByDsl<Expr>>::Output;

    /// Represents the return type of [`.having(predicate)`](crate::prelude::QueryDsl::having)
    pub type Having<Source, Predicate> = <Source as HavingDsl<Predicate>>::Output;

    /// Represents the return type of [`.union(rhs)`](crate::prelude::CombineDsl::union)
    pub type Union<Source, Rhs> = CombinationClause<
        combination_clause::Union,
        combination_clause::Distinct,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of [`.union_all(rhs)`](crate::prelude::CombineDsl::union_all)
    pub type UnionAll<Source, Rhs> = CombinationClause<
        combination_clause::Union,
        combination_clause::All,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of [`.intersect(rhs)`](crate::prelude::CombineDsl::intersect)
    pub type Intersect<Source, Rhs> = CombinationClause<
        combination_clause::Intersect,
        combination_clause::Distinct,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of [`.intersect_all(rhs)`](crate::prelude::CombineDsl::intersect_all)
    pub type IntersectAll<Source, Rhs> = CombinationClause<
        combination_clause::Intersect,
        combination_clause::All,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of [`.except(rhs)`](crate::prelude::CombineDsl::except)
    pub type Except<Source, Rhs> = CombinationClause<
        combination_clause::Except,
        combination_clause::Distinct,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of [`.except_all(rhs)`](crate::prelude::CombineDsl::except_all)
    pub type ExceptAll<Source, Rhs> = CombinationClause<
        combination_clause::Except,
        combination_clause::All,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    type JoinQuerySource<Left, Right, Kind, On> = joins::JoinOn<joins::Join<Left, Right, Kind>, On>;

    /// A query source representing the inner join between two tables.
    /// For example, for the inner join between three tables that implement `JoinTo`:
    /// `InnerJoinQuerySource<InnerJoinQuerySource<table1, table2>, table3>`
    /// Which conveniently lets you omit the exact join condition.
    pub type InnerJoinQuerySource<Left, Right, On = <Left as joins::JoinTo<Right>>::OnClause> =
        JoinQuerySource<Left, Right, joins::Inner, On>;

    /// A query source representing the left outer join between two tables.
    /// For example, for the left join between three tables that implement `JoinTo`:
    /// `LeftJoinQuerySource<LeftJoinQuerySource<table1, table2>, table3>`
    /// Which conveniently lets you omit the exact join condition.
    pub type LeftJoinQuerySource<Left, Right, On = <Left as joins::JoinTo<Right>>::OnClause> =
        JoinQuerySource<Left, Right, joins::LeftOuter, On>;

    /// [`Iterator`](std::iter::Iterator) of [`QueryResult<U>`](crate::result::QueryResult)
    ///
    /// See [`RunQueryDsl::load_iter`] for more information
    pub type LoadIter<'conn, 'query, Q, Conn, U> =
        <Q as load_dsl::LoadQueryGatWorkaround<'conn, 'query, Conn, U>>::Ret;

    /// Maps `F` to `Alias<S>`
    ///
    /// Any column `F` that belongs to `S::Table` will be transformed into
    /// [`AliasedField<S, Self>`](crate::query_source::AliasedField)
    ///
    /// Any column `F` that does not belong to `S::Table` will be left untouched.
    ///
    /// This also works with tuples and some expressions.
    pub type AliasedFields<S, F> = <F as aliasing::FieldAliasMapper<S>>::Out;
}

pub mod prelude {
    //! Re-exports important traits and types. Meant to be glob imported when using Diesel.

    #[doc(inline)]
    pub use crate::associations::{Associations, GroupedBy, Identifiable};
    #[doc(inline)]
    pub use crate::connection::Connection;
    #[doc(inline)]
    pub use crate::deserialize::{Queryable, QueryableByName};
    #[doc(inline)]
    pub use crate::expression::{
        AppearsOnTable, BoxableExpression, Expression, IntoSql, Selectable, SelectableExpression,
    };

    #[doc(inline)]
    pub use crate::expression::functions::sql_function;

    #[doc(inline)]
    pub use crate::expression::SelectableHelper;
    #[doc(inline)]
    pub use crate::expression_methods::*;
    #[doc(inline)]
    pub use crate::insertable::Insertable;
    #[doc(inline)]
    pub use crate::macros::prelude::*;
    #[doc(inline)]
    pub use crate::query_builder::AsChangeset;
    #[doc(inline)]
    pub use crate::query_builder::DecoratableTarget;
    #[doc(inline)]
    pub use crate::query_dsl::{
        BelongingToDsl, CombineDsl, JoinOnDsl, QueryDsl, RunQueryDsl, SaveChangesDsl,
    };
    #[doc(inline)]
    pub use crate::query_source::{Column, JoinTo, QuerySource, Table};
    #[doc(inline)]
    pub use crate::result::{ConnectionError, ConnectionResult, OptionalExtension, QueryResult};

    #[cfg(feature = "mysql")]
    #[doc(inline)]
    pub use crate::mysql::MysqlConnection;
    #[cfg(feature = "postgres")]
    #[doc(inline)]
    pub use crate::pg::PgConnection;
    #[cfg(feature = "sqlite")]
    #[doc(inline)]
    pub use crate::sqlite::SqliteConnection;
}

pub use crate::prelude::*;
#[doc(inline)]
pub use crate::query_builder::debug_query;
#[doc(inline)]
pub use crate::query_builder::functions::{
    delete, insert_into, insert_or_ignore_into, replace_into, select, sql_query, update,
};
pub use crate::result::Error::NotFound;

pub(crate) mod diesel {
    pub(crate) use super::*;
}

// workaround https://github.com/rust-lang/rust/pull/52234
#[doc(hidden)]
pub use __diesel_check_column_count_internal as __diesel_check_column_count;
