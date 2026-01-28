use crate::dsl::AsExpr;
use crate::expression::Expression;
use crate::expression::grouped::Grouped;
use crate::expression_methods::JsonIndex;

/// The return type of `lhs.is(rhs)`.
pub type Is<Lhs, Rhs> = Grouped<super::operators::Is<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.is_not(rhs)`.
pub type IsNot<Lhs, Rhs> = Grouped<super::operators::IsNot<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.retrieve_as_object(rhs)`](super::expression_methods::SqliteAnyJsonExpressionMethods::retrieve_as_object)
///
/// Note: SQLite's `->` operator always returns JSON (TEXT representation), not JSONB
#[cfg(feature = "sqlite")]
pub type RetrieveAsObjectSqlite<Lhs, Rhs> = Grouped<
    crate::sqlite::expression::operators::RetrieveAsObjectSqlite<
        Lhs,
        crate::dsl::AsExprOf<
            <Rhs as JsonIndex>::Expression,
            <<Rhs as JsonIndex>::Expression as Expression>::SqlType,
        >,
    >,
>;

#[doc(inline)]
pub use super::return_type_helpers::*;
