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
//! So Diesel is able to validate your queries at compile time,
//! it requires you to specify your schema in your code,
//! which you can do with [the `table!` macro][`table!`].
//! `diesel print-schema` or `infer_schema!` can be used
//! to automatically generate these macro calls
//! (by connecting to your database and querying its schema).
//!
//! [`table!`]: macro.table.html
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
//! [`update`]: fn.update.html
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
//! [`sql_function!`]: macro.sql_function.html
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
//! [`Queryable`]: deserialize/trait.Queryable.html
//! [`diesel::sql_types::Integer`]: sql_types/struct.Integer.html
//! [`ToSql`]: serialize/trait.ToSql.html
//! [`FromSql`]: deserialize/trait.FromSql.html
//!
//! ## Getting help
//!
//! If you run into problems, Diesel has a very active Gitter room.
//! You can come ask for help at
//! [gitter.im/diesel-rs/diesel](https://gitter.im/diesel-rs/diesel)

#![cfg_attr(
    feature = "large-tables",
    deprecated(
        since = "1.2.0", note = "The large-tables feature has been renamed to 32-column-tables"
    )
)]
#![cfg_attr(
    feature = "huge-tables",
    deprecated(
        since = "1.2.0", note = "The huge-tables feature has been renamed to 64-column-tables"
    )
)]
#![cfg_attr(
    feature = "x32-column-tables",
    deprecated(
        since = "1.2.1",
        note = "The x32-column-tables feature has been reanmed to 32-column-tables. The x was a workaround for a bug in crates.io that has since been resolved"
    )
)]
#![cfg_attr(
    feature = "x64-column-tables",
    deprecated(
        since = "1.2.1",
        note = "The x64-column-tables feature has been reanmed to 64-column-tables. The x was a workaround for a bug in crates.io that has since been resolved"
    )
)]
#![cfg_attr(
    feature = "x128-column-tables",
    deprecated(
        since = "1.2.1",
        note = "The x128-column-tables feature has been reanmed to 128-column-tables. The x was a workaround for a bug in crates.io that has since been resolved"
    )
)]
#![cfg_attr(feature = "unstable", feature(specialization, try_from))]
// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations, missing_docs)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "clippy",
    allow(
        option_map_unwrap_or_else, option_map_unwrap_or, match_same_arms, type_complexity,
        redundant_field_names
    )
)]
#![cfg_attr(
    feature = "clippy",
    warn(
        option_unwrap_used, result_unwrap_used, print_stdout, wrong_pub_self_convention, mut_mut,
        non_ascii_literal, similar_names, unicode_not_nfc, enum_glob_use, if_not_else,
        items_after_statements, used_underscore_binding
    )
)]
#![cfg_attr(all(test, feature = "clippy"), allow(option_unwrap_used, result_unwrap_used))]

#[cfg(feature = "postgres")]
#[macro_use]
extern crate bitflags;
extern crate byteorder;
// This is required to make `diesel_derives` re-export, but clippy thinks its unused
#[cfg_attr(feature = "clippy", allow(useless_attribute))]
#[allow(unused_imports)]
#[macro_use]
extern crate diesel_derives;
#[doc(hidden)]
pub use diesel_derives::*;

#[macro_use]
mod macros;

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
#[macro_use]
pub mod sql_types;
pub mod migration;
pub mod row;
pub mod types;

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod sqlite;

mod type_impls;
mod util;

pub mod dsl {
    //! Includes various helper types and bare functions which are named too
    //! generically to be included in prelude, but are often used when using Diesel.

    #[doc(inline)]
    pub use helper_types::*;

    #[doc(inline)]
    pub use expression::dsl::*;

    #[doc(inline)]
    pub use query_builder::functions::{delete, insert_into, insert_or_ignore_into, replace_into,
                                       select, sql_query, update};
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
    use super::query_builder::locking_clause as lock;
    use super::query_dsl::methods::*;
    use super::query_dsl::*;
    use super::query_source::joins;

    #[doc(inline)]
    pub use expression::helper_types::*;

    /// Represents the return type of `.select(selection)`
    pub type Select<Source, Selection> = <Source as SelectDsl<Selection>>::Output;

    /// Represents the return type of `.filter(predicate)`
    pub type Filter<Source, Predicate> = <Source as FilterDsl<Predicate>>::Output;

    /// Represents the return type of `.filter(lhs.eq(rhs))`
    pub type FindBy<Source, Column, Value> = Filter<Source, Eq<Column, Value>>;

    /// Represents the return type of `.for_update()`
    #[cfg(feature = "with-deprecated")]
    #[allow(deprecated)]
    pub type ForUpdate<Source> = <Source as ForUpdateDsl>::Output;

    /// Represents the return type of `.for_update()`
    #[cfg(not(feature = "with-deprecated"))]
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

    /// Represents the return type of `.left_join(rhs)`
    pub type LeftJoin<Source, Rhs> =
        <Source as JoinWithImplicitOnClause<Rhs, joins::LeftOuter>>::Output;

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

    /// Represents the return type of `.aliased(alias)`
    #[cfg(diesel_experimental)]
    pub type Aliased<Source, Alias> = <Source as AliasedDsl<Alias>>::Output;
}

pub mod prelude {
    //! Re-exports important traits and types. Meant to be glob imported when using Diesel.
    pub use associations::{GroupedBy, Identifiable};
    pub use connection::Connection;
    #[deprecated(since = "1.1.0", note = "Explicitly `use diesel::deserialize::Queryable")]
    pub use deserialize::Queryable;
    pub use expression::{AppearsOnTable, BoxableExpression, Expression, IntoSql,
                         SelectableExpression};
    pub use expression_methods::*;
    #[doc(inline)]
    pub use insertable::Insertable;
    #[doc(hidden)]
    pub use query_dsl::GroupByDsl;
    pub use query_dsl::{BelongingToDsl, JoinOnDsl, QueryDsl, RunQueryDsl, SaveChangesDsl};

    pub use query_source::{Column, JoinTo, QuerySource, Table};
    pub use result::{ConnectionError, ConnectionResult, OptionalExtension, QueryResult};

    #[cfg(feature = "mysql")]
    pub use mysql::MysqlConnection;
    #[cfg(feature = "postgres")]
    pub use pg::PgConnection;
    #[cfg(feature = "sqlite")]
    pub use sqlite::SqliteConnection;
}

pub use prelude::*;
#[doc(inline)]
pub use query_builder::debug_query;
#[doc(inline)]
pub use query_builder::functions::{delete, insert_into, insert_or_ignore_into, replace_into,
                                   select, sql_query, update};
pub use result::Error::NotFound;
