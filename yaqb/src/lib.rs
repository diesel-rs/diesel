#![deny(warnings)]
pub mod expression;
pub mod persistable;
pub mod types;

mod connection;
mod db_result;
pub mod query_builder;
mod query_dsl;
pub mod query_source;
pub mod result;
mod row;

pub mod helper_types {
    use super::query_dsl::*;
    use super::expression::helper_types::Eq;

    pub type Select<Source, Selection, Type = <Selection as super::Expression>::SqlType> =
        <Source as SelectDsl<Selection, Type>>::Output;

    pub type Filter<Source, Predicate> =
        <Source as FilterDsl<Predicate>>::Output;

    pub type FindBy<Source, Column, Value> =
        Filter<Source, Eq<Column, Value>>;

    pub type Order<Source, Ordering> =
        <Source as OrderDsl<Ordering>>::Output;

    pub type Limit<Source> = <Source as LimitDsl>::Output;

    pub type Offset<Source> = <Source as OffsetDsl>::Output;

    pub type With<'a, Source, Other> = <Source as WithDsl<'a, Other>>::Output;
}

#[macro_use]
mod macros;

pub use connection::{Connection, Cursor};
pub use expression::{Expression, SelectableExpression, BoxableExpression};
pub use expression::expression_methods::*;
pub use query_dsl::*;
pub use query_source::{QuerySource, Queriable, Table, Column, JoinTo};
pub use result::{QueryResult, TransactionError, TransactionResult, ConnectionError, ConnectionResult};
