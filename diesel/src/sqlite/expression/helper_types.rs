use crate::dsl::AsExpr;
use crate::expression::grouped::Grouped;
use crate::expression::Expression;

/// The return type of `lhs.is(rhs)`.
pub type Is<Lhs, Rhs> = Grouped<super::operators::Is<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.is_not(rhs)`.
pub type IsNot<Lhs, Rhs> = Grouped<super::operators::IsNot<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.retrieve_as_object(rhs)`](super::expression_methods::SqliteAnyJsonExpressionMethods::retrieve_as_object)
///
/// Note: SQLite's `->` operator always returns JSON (TEXT representation), not JSONB
#[cfg(feature = "sqlite")]
pub type RetrieveAsObjectJson<Lhs, Rhs, ST> =
    Grouped<super::operators::RetrieveAsObjectJson<Lhs, crate::dsl::AsExprOf<Rhs, ST>>>;

#[doc(hidden)] // needed for `#[auto_type]`
#[cfg(feature = "sqlite")]
pub type RetrieveAsObject<Lhs, Rhs> = RetrieveAsObjectJson<
    Lhs,
    <Rhs as crate::sqlite::expression::expression_methods::JsonIndex>::Expression,
    <<Rhs as crate::sqlite::expression::expression_methods::JsonIndex>::Expression as Expression>::SqlType,
>;

/// The return type of [`lhs.retrieve_as_text(rhs)`](super::expression_methods::SqliteAnyJsonExpressionMethods::retrieve_as_text)
#[cfg(feature = "sqlite")]
pub type RetrieveAsTextJson<Lhs, Rhs, ST> =
    Grouped<super::operators::RetrieveAsTextJson<Lhs, crate::dsl::AsExprOf<Rhs, ST>>>;

#[doc(hidden)] // needed for `#[auto_type]`
#[cfg(feature = "sqlite")]
pub type RetrieveAsText<Lhs, Rhs> = RetrieveAsTextJson<
    Lhs,
    <Rhs as crate::sqlite::expression::expression_methods::JsonIndex>::Expression,
    <<Rhs as crate::sqlite::expression::expression_methods::JsonIndex>::Expression as Expression>::SqlType,
>;

#[doc(inline)]
pub use super::return_type_helpers::*;
