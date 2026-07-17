#[doc(hidden)]
pub use crate::expression::nullable::Nullable as NullableExpression;
#[doc(hidden)]
#[cfg(feature = "postgres_backend")]
pub use crate::pg::query_builder::tablesample::TablesampleMethod;
#[doc(hidden)]
pub use crate::query_builder::from_clause::{FromClause, NoFromClause};
#[doc(hidden)]
pub use crate::query_builder::nodes::{
    Identifier, InfixNode, StaticQueryFragment, StaticQueryFragmentInstance,
};
#[doc(hidden)]
pub mod returning {
    #[doc(hidden)]
    pub use crate::query_builder::returning::*;
}
#[doc(hidden)]
pub use crate::query_builder::select_statement::SelectStatement;
#[doc(hidden)]
pub use crate::query_builder::select_statement::boxed::BoxedSelectStatement;
#[doc(hidden)]
pub use crate::query_source::aliasing::{
    AliasAliasAppearsInFromClause, AliasAliasAppearsInFromClauseSameTable,
    AliasAppearsInFromClause, FieldAliasMapperAssociatedTypesDisjointnessTrick,
};
#[doc(hidden)]
pub use crate::query_source::joins::{Inner, Join, JoinOn, LeftOuter};
#[doc(hidden)]
pub use crate::query_source::private::{Pick, PlainQuerySource, Sealed};

#[doc(hidden)]
pub mod ops {
    #[doc(hidden)]
    pub use crate::expression::ops::numeric::*;
}

#[doc(hidden)]
pub use crate::expand_pg;
#[doc(hidden)]
#[cfg(feature = "custom-count-column-tables")]
pub const MAX_COLUMN_COUNT: u16 = {
    let number = env!("DIESEL_MAX_COLUMN_COUNT");
    let Ok(n) = u16::from_str_radix(number, 10) else {
        panic!("DIESEL_MAX_COLUMN_COUNT is a number that fits into a u16");
    };
    n
};
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    feature = "128-column-tables"
))]
pub const MAX_COLUMN_COUNT: u16 = 128;
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    not(feature = "128-column-tables"),
    feature = "64-column-tables"
))]
pub const MAX_COLUMN_COUNT: u16 = 64;
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    not(feature = "128-column-tables"),
    not(feature = "64-column-tables"),
    feature = "32-column-tables"
))]
pub const MAX_COLUMN_COUNT: u16 = 32;
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    not(feature = "128-column-tables"),
    not(feature = "64-column-tables"),
    not(feature = "32-column-tables")
))]
pub const MAX_COLUMN_COUNT: u16 = 16;
