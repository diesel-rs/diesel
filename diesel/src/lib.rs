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
//! If you run into problems, Diesel has a very active Gitter room.
//! You can come ask for help at
//! [gitter.im/diesel-rs/diesel](https://gitter.im/diesel-rs/diesel)

#![cfg_attr(feature = "unstable", feature(specialization, trait_alias))]
// For the `specialization` feature.
#![cfg_attr(feature = "unstable", allow(incomplete_features))]
// Built-in Lints
#![deny(warnings)]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
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
    clippy::wrong_pub_self_convention,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::enum_glob_use,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]

#[cfg(feature = "postgres")]
#[macro_use]
extern crate bitflags;
extern crate byteorder;
extern crate diesel_derives;

#[macro_use]
#[doc(hidden)]
pub mod macros;

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

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod sqlite;

mod type_impls;
mod util;

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(since = "2.0.0", note = "Use explicit macro imports instead")]
pub use diesel_derives::*;

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
    use super::query_builder::locking_clause as lock;
    use super::query_builder::AsQuery;
    use super::query_dsl::methods::*;
    use super::query_dsl::*;
    use super::query_source::joins;

    #[doc(inline)]
    pub use crate::expression::helper_types::*;

    /// Represents the return type of `.select(selection)`
    pub type Select<Source, Selection> = <Source as SelectDsl<Selection>>::Output;

    /// Represents the return type of `.filter(predicate)`
    pub type Filter<Source, Predicate> = <Source as FilterDsl<Predicate>>::Output;

    /// Represents the return type of `.filter(lhs.eq(rhs))`
    pub type FindBy<Source, Column, Value> = Filter<Source, Eq<Column, Value>>;

    /// Represents the return type of `.for_update()`
    pub type ForUpdate<Source> = <Source as LockingDsl<lock::ForUpdate>>::Output;

    /// Represents the return type of `.for_no_key_update()`
    pub type ForNoKeyUpdate<Source> = <Source as LockingDsl<lock::ForNoKeyUpdate>>::Output;

    /// Represents the return type of `.for_share()`
    pub type ForShare<Source> = <Source as LockingDsl<lock::ForShare>>::Output;

    /// Represents the return type of `.for_key_share()`
    pub type ForKeyShare<Source> = <Source as LockingDsl<lock::ForKeyShare>>::Output;

    /// Represents the return type of `.skip_locked()`
    pub type SkipLocked<Source> = <Source as ModifyLockDsl<lock::SkipLocked>>::Output;

    /// Represents the return type of `.no_wait()`
    pub type NoWait<Source> = <Source as ModifyLockDsl<lock::NoWait>>::Output;

    /// Represents the return type of `.find(pk)`
    pub type Find<Source, PK> = <Source as FindDsl<PK>>::Output;

    /// Represents the return type of `.or_filter(predicate)`
    pub type OrFilter<Source, Predicate> = <Source as OrFilterDsl<Predicate>>::Output;

    /// Represents the return type of `.order(ordering)`
    pub type Order<Source, Ordering> = <Source as OrderDsl<Ordering>>::Output;

    /// Represents the return type of `.then_order_by(ordering)`
    pub type ThenOrderBy<Source, Ordering> = <Source as ThenOrderDsl<Ordering>>::Output;

    /// Represents the return type of `.limit()`
    pub type Limit<Source> = <Source as LimitDsl>::Output;

    /// Represents the return type of `.offset()`
    pub type Offset<Source> = <Source as OffsetDsl>::Output;

    /// Represents the return type of `.inner_join(rhs)`
    pub type InnerJoin<Source, Rhs> =
        <Source as JoinWithImplicitOnClause<Rhs, joins::Inner>>::Output;

    /// Represents the return type of `.inner_join(rhs.on(on))`
    pub type InnerJoinOn<Source, Rhs, On> =
        <Source as InternalJoinDsl<Rhs, joins::Inner, On>>::Output;

    /// Represents the return type of `.left_join(rhs)`
    pub type LeftJoin<Source, Rhs> =
        <Source as JoinWithImplicitOnClause<Rhs, joins::LeftOuter>>::Output;

    /// Represents the return type of `.left_join(rhs.on(on))`
    pub type LeftJoinOn<Source, Rhs, On> =
        <Source as InternalJoinDsl<Rhs, joins::LeftOuter, On>>::Output;

    use super::associations::HasTable;
    use super::query_builder::{AsChangeset, IntoUpdateTarget, UpdateStatement};

    /// Represents the return type of `update(lhs).set(rhs)`
    pub type Update<Target, Changes> = UpdateStatement<
        <Target as HasTable>::Table,
        <Target as IntoUpdateTarget>::WhereClause,
        <Changes as AsChangeset>::Changeset,
    >;

    /// Represents the return type of `.into_boxed::<'a, DB>()`
    pub type IntoBoxed<'a, Source, DB> = <Source as BoxedDsl<'a, DB>>::Output;

    /// Represents the return type of `.distinct()`
    pub type Distinct<Source> = <Source as DistinctDsl>::Output;

    /// Represents the return type of `.distinct_on(expr)`
    #[cfg(feature = "postgres")]
    pub type DistinctOn<Source, Expr> = <Source as DistinctOnDsl<Expr>>::Output;

    /// Represents the return type of `.single_value()`
    pub type SingleValue<Source> = <Source as SingleValueDsl>::Output;

    /// Represents the return type of `.nullable()`
    pub type NullableSelect<Source> = <Source as SelectNullableDsl>::Output;

    /// Represents the return type of `.group_by(expr)`
    pub type GroupBy<Source, Expr> = <Source as GroupByDsl<Expr>>::Output;

    /// Represents the return type of `.having(predicate)`
    pub type Having<Source, Predicate> = <Source as HavingDsl<Predicate>>::Output;

    /// Represents the return type of `.union(rhs)`
    pub type Union<Source, Rhs> = CombinationClause<
        combination_clause::Union,
        combination_clause::Distinct,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of `.union_all(rhs)`
    pub type UnionAll<Source, Rhs> = CombinationClause<
        combination_clause::Union,
        combination_clause::All,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of `.intersect(rhs)`
    pub type Intersect<Source, Rhs> = CombinationClause<
        combination_clause::Intersect,
        combination_clause::Distinct,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of `.intersect_all(rhs)`
    pub type IntersectAll<Source, Rhs> = CombinationClause<
        combination_clause::Intersect,
        combination_clause::All,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of `.except(rhs)`
    pub type Except<Source, Rhs> = CombinationClause<
        combination_clause::Except,
        combination_clause::Distinct,
        <Source as CombineDsl>::Query,
        <Rhs as AsQuery>::Query,
    >;

    /// Represents the return type of `.except_all(rhs)`
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

    pub use crate::expression::SelectableHelper;

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
    pub use super::*;
}
