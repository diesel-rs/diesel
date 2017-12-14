//! # Diesel
//!
//! Diesel is an ORM and query builder designed to reduce the boilerplate for database interactions.
//!
//! * [Homepage](https://diesel.rs/)
//! * [Getting Started Guide](https://diesel.rs/guides/getting-started/)
//! * [Source Code](https://github.com/diesel-rs/diesel/)
//!
//! ## Crates
//!
//! * **Diesel**: core libary, documented here
//! * [**Diesel CLI**](https://github.com/diesel-rs/diesel/tree/master/diesel_cli): command line tool that aids in managing your database schema
//!
#![cfg_attr(feature = "unstable", feature(specialization))]
// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "clippy",
            allow(option_map_unwrap_or_else, option_map_unwrap_or, match_same_arms,
                  type_complexity))]
#![cfg_attr(feature = "clippy",
            warn(option_unwrap_used, result_unwrap_used, print_stdout,
                 wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, enum_glob_use, if_not_else, items_after_statements,
                 used_underscore_binding))]
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

#[deny(missing_docs)]
pub mod associations;
pub mod backend;
pub mod connection;
#[macro_use]
pub mod expression;
pub mod expression_methods;
#[doc(hidden)]
pub mod insertable;
pub mod query_builder;
#[deny(missing_docs)]
#[macro_use]
pub mod types;

#[cfg(feature = "mysql")]
#[deny(missing_docs)]
pub mod mysql;
#[cfg(feature = "postgres")]
#[deny(missing_docs)]
pub mod pg;
#[cfg(feature = "sqlite")]
#[deny(missing_docs)]
pub mod sqlite;

#[deny(missing_docs)]
pub mod query_dsl;
#[deny(missing_docs)]
pub mod query_source;
#[deny(missing_docs)]
pub mod result;
#[deny(missing_docs)]
pub mod row;
mod util;

pub mod dsl {
    //! Includes various helper types and bare functions which are named too
    //! generically to be included in prelude, but are often used when using Diesel.

    #[doc(inline)]
    pub use helper_types::*;

    #[doc(inline)]
    pub use expression::dsl::*;
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
    use super::query_dsl::*;
    use super::query_dsl::methods::*;
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
    pub type ForUpdate<Source> = <Source as ForUpdateDsl>::Output;

    /// Represents the return type of `.find(pk)`
    pub type Find<Source, PK> = <Source as FindDsl<PK>>::Output;

    /// Represents the return type of `.order(ordering)`
    pub type Order<Source, Ordering> = <Source as OrderDsl<Ordering>>::Output;

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
}

pub mod prelude {
    //! Re-exports important traits and types. Meant to be glob imported when using Diesel.
    pub use associations::{GroupedBy, Identifiable};
    pub use connection::Connection;
    pub use expression::{AppearsOnTable, BoxableExpression, Expression, IntoSql,
                         SelectableExpression};
    pub use expression_methods::*;
    #[doc(inline)]
    pub use insertable::Insertable;
    #[doc(hidden)]
    pub use query_dsl::GroupByDsl;
    pub use query_dsl::{BelongingToDsl, JoinOnDsl, QueryDsl, RunQueryDsl, SaveChangesDsl};

    pub use query_source::{Column, JoinTo, QuerySource, Queryable, Table};
    pub use result::{ConnectionError, ConnectionResult, OptionalExtension, QueryResult};

    #[cfg(feature = "postgres")]
    pub use pg::PgConnection;
    #[cfg(feature = "sqlite")]
    pub use sqlite::SqliteConnection;
    #[cfg(feature = "mysql")]
    pub use mysql::MysqlConnection;
}

pub use prelude::*;
#[doc(inline)]
pub use query_builder::debug_query;
#[doc(inline)]
pub use query_builder::functions::{delete, insert_into, replace_into, select, sql_query, update};
pub use result::Error::NotFound;
#[doc(inline)]
pub use types::structs::data_types;
