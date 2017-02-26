//! Diesel is an ORM and query builder designed to reduce the boilerplate for database
//! interactions. [A getting started guide](http://diesel.rs/guides/getting-started/) can be
//! found on our website.
#![cfg_attr(feature = "unstable", feature(specialization))]

// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]

// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file="../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(
    option_map_unwrap_or_else, option_map_unwrap_or,
    match_same_arms, type_complexity,
))]
#![cfg_attr(feature = "clippy", warn(
    option_unwrap_used, result_unwrap_used, print_stdout, wrong_pub_self_convention,
    mut_mut, non_ascii_literal, similar_names, unicode_not_nfc,
    enum_glob_use, if_not_else, items_after_statements, used_underscore_binding,
))]
#![cfg_attr(all(test, feature = "clippy"), allow(result_unwrap_used))]

extern crate byteorder;

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
#[macro_use]
pub mod expression;
#[doc(hidden)]
pub mod insertable;
pub mod query_builder;
#[macro_use]
pub mod types;

#[deprecated(since = "0.10.0", note = "use `insertable` instead")]
#[cfg(feature = "with-deprecated")]
pub use self::insertable as persistable;

#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub mod migrations;
mod query_dsl;
pub mod query_source;
pub mod result;
pub mod row;

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
    use super::expression::helper_types::Eq;

    /// Represents the return type of `.select(selection)`
    pub type Select<Source, Selection, Type = <Selection as super::Expression>::SqlType> =
        <Source as SelectDsl<Selection, Type>>::Output;

    /// Represents the return type of `.filter(predicate)`
    pub type Filter<Source, Predicate> =
        <Source as FilterDsl<Predicate>>::Output;

    /// Represents the return type of `.filter(lhs.eq(rhs))`
    pub type FindBy<Source, Column, Value> =
        Filter<Source, Eq<Column, Value>>;

    /// Represents the return type of `.find(pk)`
    pub type Find<Source, PK> = <Source as FindDsl<PK>>::Output;

    /// Represents the return type of `.order(ordering)`
    pub type Order<Source, Ordering> =
        <Source as OrderDsl<Ordering>>::Output;

    /// Represents the return type of `.limit()`
    pub type Limit<Source> = <Source as LimitDsl>::Output;

    /// Represents the return type of `.offset()`
    pub type Offset<Source> = <Source as OffsetDsl>::Output;

    use super::associations::HasTable;
    use super::query_builder::{UpdateStatement, IntoUpdateTarget, AsChangeset};
    /// Represents the return type of `update(lhs).set(rhs)`
    pub type Update<Target, Changes> = UpdateStatement<
        <Target as HasTable>::Table,
        <Target as IntoUpdateTarget>::WhereClause,
        <Changes as AsChangeset>::Changeset,
    >;
}

pub mod prelude {
    //! Re-exports important traits and types. Meant to be glob imported when using Diesel.
    pub use associations::GroupedBy;
    pub use connection::Connection;
    pub use expression::{Expression, SelectableExpression, AppearsOnTable, BoxableExpression};
    pub use expression::expression_methods::*;
    #[doc(inline)]
    pub use insertable::Insertable;
    pub use query_dsl::*;
    pub use query_source::{QuerySource, Queryable, Table, Column, JoinTo};
    pub use result::{QueryResult, ConnectionError, ConnectionResult, OptionalExtension};
}

pub use prelude::*;
#[doc(inline)]
pub use query_builder::functions::{insert, update, delete, select};
#[cfg(feature = "sqlite")]
pub use sqlite::query_builder::functions::*;
pub use result::Error::NotFound;
#[doc(inline)]
pub use types::structs::data_types;
