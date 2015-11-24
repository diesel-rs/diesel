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
    pub use super::query_dsl::{
        FilterOutput as Filter,
        FindByOutput as FindBy,
        LimitOutput as Limit,
        OrderOutput as Order,
    };
}

#[macro_use]
mod macros;

pub use connection::{Connection, Cursor};
pub use expression::{Expression, SelectableExpression};
pub use query_dsl::*;
pub use query_source::{QuerySource, Queriable, Table, Column, JoinTo};
pub use result::{TransactionError, TransactionResult, ConnectionError, ConnectionResult};
