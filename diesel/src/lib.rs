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
//!   with [the `define_sql_function!` macro][`define_sql_function!`].
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
//! ## How to read diesels compile time error messages
//!
//! Diesel is known for generating large complicated looking errors. Usually
//! most of these error messages can be broken down easily. The following
//! section tries to give an overview of common error messages and how to read them.
//! As a general note it's always useful to read the complete error message as emitted
//! by rustc, including the `required because of …` part of the message.
//! Your IDE might hide important parts!
//!
//! The following error messages are common:
//!
//! * `the trait bound (diesel::sql_types::Integer, …, diesel::sql_types::Text): load_dsl::private::CompatibleType<YourModel, Pg> is not satisfied`
//!    while trying to execute a query:
//!    This error indicates a mismatch between what your query returns and what your model struct
//!    expects the query to return. The fields need to match in terms of field order, field type
//!    and field count. If you are sure that everything matches, double check the enabled diesel
//!    features (for support for types from other crates) and double check (via `cargo tree`)
//!    that there is only one version of such a shared crate in your dependency tree.
//!    Consider using [`#[derive(Selectable)]`](derive@crate::prelude::Selectable) +
//!    `#[diesel(check_for_backend(diesel::pg::Pg))]`
//!    to improve the generated error message.
//! * `the trait bound i32: diesel::Expression is not satisfied` in the context of `Insertable`
//!    model structs:
//!    This error indicates a type mismatch between the field you are trying to insert into the database
//!    and the actual database type. These error messages contain a line
//!    like ` = note: required for i32 to implement AsExpression<diesel::sql_types::Text>`
//!    that show both the provided rust side type (`i32` in that case) and the expected
//!    database side type (`Text` in that case).
//! * `the trait bound i32: AppearsOnTable<users::table> is not satisfied` in the context of `AsChangeset`
//!    model structs:
//!    This error indicates a type mismatch between the field you are trying to update and the actual
//!    database type. Double check your type mapping.
//! * `the trait bound SomeLargeType: QueryFragment<Sqlite, SomeMarkerType> is not satisfied` while
//!    trying to execute a query.
//!    This error message indicates that a given query is not supported by your backend. This usually
//!    means that you are trying to use SQL features from one SQL dialect on a different database
//!    system. Double check your query that everything required is supported by the selected
//!    backend. If that's the case double check that the relevant feature flags are enabled
//!    (for example, `returning_clauses_for_sqlite_3_35` for enabling support for returning clauses in newer
//!    sqlite versions)
//! * `the trait bound posts::title: SelectableExpression<users::table> is not satisfied` while
//!    executing a query:
//!    This error message indicates that you're trying to select a field from a table
//!    that does not appear in your from clause. If your query joins the relevant table via
//!    [`left_join`](crate::query_dsl::QueryDsl::left_join) you need to call
//!    [`.nullable()`](crate::expression_methods::NullableExpressionMethods::nullable)
//!    on the relevant column in your select clause.
//!
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
//! - `sqlite`: This feature enables the diesel sqlite backend. Enabling this feature requires per default
//! a compatible copy of `libsqlite3` for your target architecture. Alternatively, you can add `libsqlite3-sys`
//! with the `bundled` feature as a dependency to your crate so SQLite will be bundled:
//! ```toml
//! [dependencies]
//! libsqlite3-sys = { version = "0.25.2", features = ["bundled"] }
//! ```
//! - `postgres`: This feature enables the diesel postgres backend. Enabling this feature requires a compatible
//! copy of `libpq` for your target architecture. This features implies `postgres_backend`
//! - `mysql`: This feature enables the idesel mysql backend. Enabling this feature requires a compatible copy
//! of `libmysqlclient` for your target architecture. This feature implies `mysql_backend`
//! - `postgres_backend`: This feature enables those parts of diesels postgres backend, that are not dependent
//! on `libpq`. Diesel does not provide any connection implementation with only this feature enabled.
//! This feature can be used to implement a custom implementation of diesels `Connection` trait for the
//! postgres backend outside of diesel itself, while reusing the existing query dsl extensions for the
//! postgres backend
//! - `mysql_backend`: This feature enables those parts of diesels mysql backend, that are not dependent
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
//! or write a custom [`Connection`] implementation. **Do not use this feature for
//! any other usecase**. By enabling this feature you explicitly opt out diesel stability guarantees. We explicitly
//! reserve us the right to break API's exported under this feature flag in any upcoming minor version release.
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
//! - `extras`: This feature enables the feature flagged support for any third party crate. This implies the
//! following feature flags: `serde_json`, `chrono`, `uuid`, `network-address`, `numeric`, `r2d2`
//! - `with-deprecated`: This feature enables items marked as `#[deprecated]`. It is enabled by default.
//! disabling this feature explicitly opts out diesels stability guarantee.
//! - `without-deprecated`: This feature disables any item marked as `#[deprecated]`. Enabling this feature
//! explicitly opts out the stability guarantee given by diesel. This feature overrides the `with-deprecated`.
//! Note that this may also remove items that are not shown as `#[deprecated]` in our documentation, due to
//! various bugs in rustdoc. It can be used to check if you depend on any such hidden `#[deprecated]` item.
//!
//! By default the following features are enabled:
//!
//! - `with-deprecated`
//! - `32-column-tables`

#![cfg_attr(feature = "unstable", feature(trait_alias))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
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
#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::map_unwrap_or, clippy::unwrap_used))]

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

pub use diesel_derives::MultiConnection;

#[allow(unknown_lints, ambiguous_glob_reexports)]
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

    #[doc(inline)]
    #[cfg(feature = "postgres_backend")]
    pub use crate::query_builder::functions::{copy_from, copy_to};

    #[doc(inline)]
    pub use diesel_derives::auto_type;

    #[cfg(feature = "postgres_backend")]
    #[doc(inline)]
    pub use crate::pg::expression::extensions::OnlyDsl;

    #[cfg(feature = "postgres_backend")]
    #[doc(inline)]
    pub use crate::pg::expression::extensions::TablesampleDsl;
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
    #[allow(non_camel_case_types)] // required for `#[auto_type]`
    pub type select<Selection> = crate::query_builder::SelectStatement<
        crate::query_builder::NoFromClause,
        SelectClause<Selection>,
    >;

    #[doc(hidden)]
    #[deprecated(note = "Use `select` instead")]
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

    /// Represents the return type of [`.order_by(ordering)`](crate::prelude::QueryDsl::order_by)
    ///
    /// Type alias of [Order]
    pub type OrderBy<Source, Ordering> = Order<Source, Ordering>;

    /// Represents the return type of [`.then_order_by(ordering)`](crate::prelude::QueryDsl::then_order_by)
    pub type ThenOrderBy<Source, Ordering> = <Source as ThenOrderDsl<Ordering>>::Output;

    /// Represents the return type of [`.limit()`](crate::prelude::QueryDsl::limit)
    pub type Limit<Source, DummyArgForAutoType = i64> =
        <Source as LimitDsl<DummyArgForAutoType>>::Output;

    /// Represents the return type of [`.offset()`](crate::prelude::QueryDsl::offset)
    pub type Offset<Source, DummyArgForAutoType = i64> =
        <Source as OffsetDsl<DummyArgForAutoType>>::Output;

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
    ///
    /// The third generic type (`On`) controls how the tables are
    /// joined.
    ///
    /// By default, the implicit join established by [`joinable!`][]
    /// will be used, allowing you to omit the exact join
    /// condition. For example, for the inner join between three
    /// tables that implement [`JoinTo`][], you only need to specify
    /// the tables: `InnerJoinQuerySource<InnerJoinQuerySource<table1,
    /// table2>, table3>`.
    ///
    /// [`JoinTo`]: crate::query_source::JoinTo
    ///
    /// If you use an explicit `ON` clause, you will need to specify
    /// the `On` generic type.
    ///
    /// ```rust
    /// # include!("doctest_setup.rs");
    /// use diesel::{dsl, helper_types::InnerJoinQuerySource};
    /// # use diesel::{backend::Backend, serialize::ToSql, sql_types};
    /// use schema::*;
    ///
    /// # fn main() -> QueryResult<()> {
    /// #     let conn = &mut establish_connection();
    /// #
    /// // If you have an explicit join like this...
    /// let join_constraint = comments::columns::post_id.eq(posts::columns::id);
    /// #     let query =
    /// posts::table.inner_join(comments::table.on(join_constraint));
    /// #
    /// #     // Dummy usage just to ensure the example compiles.
    /// #     let filter = posts::columns::id.eq(1);
    /// #     let filter: &FilterExpression<_> = &filter;
    /// #     query.filter(filter).select(posts::columns::id).get_result::<i32>(conn)?;
    /// #
    /// #     Ok(())
    /// # }
    ///
    /// // ... you can use `InnerJoinQuerySource` like this.
    /// type JoinConstraint = dsl::Eq<comments::columns::post_id, posts::columns::id>;
    /// type MyInnerJoinQuerySource = InnerJoinQuerySource<posts::table, comments::table, JoinConstraint>;
    /// # type FilterExpression<DB> = dyn BoxableExpression<MyInnerJoinQuerySource, DB, SqlType = sql_types::Bool>;
    /// ```
    pub type InnerJoinQuerySource<Left, Right, On = <Left as joins::JoinTo<Right>>::OnClause> =
        JoinQuerySource<Left, Right, joins::Inner, On>;

    /// A query source representing the left outer join between two tables.
    ///
    /// The third generic type (`On`) controls how the tables are
    /// joined.
    ///
    /// By default, the implicit join established by [`joinable!`][]
    /// will be used, allowing you to omit the exact join
    /// condition. For example, for the left join between three
    /// tables that implement [`JoinTo`][], you only need to specify
    /// the tables: `LeftJoinQuerySource<LeftJoinQuerySource<table1,
    /// table2>, table3>`.
    ///
    /// [`JoinTo`]: crate::query_source::JoinTo
    ///
    /// If you use an explicit `ON` clause, you will need to specify
    /// the `On` generic type.
    ///
    /// ```rust
    /// # include!("doctest_setup.rs");
    /// use diesel::{dsl, helper_types::LeftJoinQuerySource};
    /// # use diesel::{backend::Backend, serialize::ToSql, sql_types};
    /// use schema::*;
    ///
    /// # fn main() -> QueryResult<()> {
    /// #     let conn = &mut establish_connection();
    /// #
    /// // If you have an explicit join like this...
    /// let join_constraint = comments::columns::post_id.eq(posts::columns::id);
    /// #     let query =
    /// posts::table.left_join(comments::table.on(join_constraint));
    /// #
    /// #     // Dummy usage just to ensure the example compiles.
    /// #     let filter = posts::columns::id.eq(1);
    /// #     let filter: &FilterExpression<_> = &filter;
    /// #     query.filter(filter).select(posts::columns::id).get_result::<i32>(conn)?;
    /// #
    /// #     Ok(())
    /// # }
    ///
    /// // ... you can use `LeftJoinQuerySource` like this.
    /// type JoinConstraint = dsl::Eq<comments::columns::post_id, posts::columns::id>;
    /// type MyLeftJoinQuerySource = LeftJoinQuerySource<posts::table, comments::table, JoinConstraint>;
    /// # type FilterExpression<DB> = dyn BoxableExpression<MyLeftJoinQuerySource, DB, SqlType = sql_types::Bool>;
    /// ```
    pub type LeftJoinQuerySource<Left, Right, On = <Left as joins::JoinTo<Right>>::OnClause> =
        JoinQuerySource<Left, Right, joins::LeftOuter, On>;

    /// Maps `F` to `Alias<S>`
    ///
    /// Any column `F` that belongs to `S::Table` will be transformed into
    /// [`AliasedField<S, Self>`](crate::query_source::AliasedField)
    ///
    /// Any column `F` that does not belong to `S::Table` will be left untouched.
    ///
    /// This also works with tuples and some expressions.
    pub type AliasedFields<S, F> = <F as aliasing::FieldAliasMapper<S>>::Out;

    #[doc(hidden)]
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    #[deprecated(note = "Use `LoadQuery::RowIter` directly")]
    pub type LoadIter<'conn, 'query, Q, Conn, U, B = crate::connection::DefaultLoadingMode> =
        <Q as load_dsl::LoadQuery<'query, Conn, U, B>>::RowIter<'conn>;

    /// Represents the return type of [`diesel::delete`]
    #[allow(non_camel_case_types)] // required for `#[auto_type]`
    pub type delete<T> = crate::query_builder::DeleteStatement<
        <T as HasTable>::Table,
        <T as IntoUpdateTarget>::WhereClause,
    >;

    /// Represents the return type of [`diesel::insert_into`]
    #[allow(non_camel_case_types)] // required for `#[auto_type]`
    pub type insert_into<T> = crate::query_builder::IncompleteInsertStatement<T>;

    /// Represents the return type of [`diesel::insert_or_ignore_into`]
    #[allow(non_camel_case_types)] // required for `#[auto_type]`
    pub type insert_or_ignore_into<T> = crate::query_builder::IncompleteInsertOrIgnoreStatement<T>;

    /// Represents the return type of [`diesel::replace_into`]
    #[allow(non_camel_case_types)] // required for `#[auto_type]`
    pub type replace_into<T> = crate::query_builder::IncompleteReplaceStatement<T>;

    /// Represents the return type of
    /// [`IncompleteInsertStatement::values()`](crate::query_builder::IncompleteInsertStatement::values)
    pub type Values<I, U> = crate::query_builder::InsertStatement<
        <I as crate::query_builder::insert_statement::InsertAutoTypeHelper>::Table,
        <U as crate::Insertable<
            <I as crate::query_builder::insert_statement::InsertAutoTypeHelper>::Table,
        >>::Values,
        <I as crate::query_builder::insert_statement::InsertAutoTypeHelper>::Op,
    >;

    /// Represents the return type of
    /// [`UpdateStatement::set()`](crate::query_builder::UpdateStatement::set)
    pub type Set<U, V> = crate::query_builder::UpdateStatement<
        <U as crate::query_builder::update_statement::UpdateAutoTypeHelper>::Table,
        <U as crate::query_builder::update_statement::UpdateAutoTypeHelper>::Where,
        <V as crate::AsChangeset>::Changeset,
    >;
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
    // If [`IntoSql`](crate::expression::helper_types::IntoSql) the type gets imported at the
    // same time as IntoSql the trait (this one) gets imported via the prelude, then
    // methods of the trait won't be resolved because the type may take priority over the trait.
    // That issue can be avoided by also importing it anonymously:
    pub use crate::expression::IntoSql as _;

    #[doc(inline)]
    pub use crate::expression::functions::define_sql_function;
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
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
    pub use crate::query_source::SizeRestrictedColumn as _;
    #[doc(inline)]
    pub use crate::query_source::{Column, JoinTo, QuerySource, Table};
    #[doc(inline)]
    pub use crate::result::{
        ConnectionError, ConnectionResult, OptionalEmptyChangesetExtension, OptionalExtension,
        QueryResult,
    };
    #[doc(inline)]
    pub use diesel_derives::table_proc as table;

    #[cfg(feature = "mysql")]
    #[doc(inline)]
    pub use crate::mysql::MysqlConnection;
    #[doc(inline)]
    #[cfg(feature = "postgres_backend")]
    pub use crate::pg::query_builder::copy::ExecuteCopyFromDsl;
    #[cfg(feature = "postgres")]
    #[doc(inline)]
    pub use crate::pg::PgConnection;
    #[cfg(feature = "sqlite")]
    #[doc(inline)]
    pub use crate::sqlite::SqliteConnection;
}

#[doc(inline)]
pub use crate::macros::table;
pub use crate::prelude::*;
#[doc(inline)]
pub use crate::query_builder::debug_query;
#[doc(inline)]
#[cfg(feature = "postgres")]
pub use crate::query_builder::functions::{copy_from, copy_to};
#[doc(inline)]
pub use crate::query_builder::functions::{
    delete, insert_into, insert_or_ignore_into, replace_into, select, sql_query, update,
};
pub use crate::result::Error::NotFound;

extern crate self as diesel;
