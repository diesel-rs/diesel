use crate::dsl;
use crate::expression::Expression;
use crate::expression::TypedExpressionType;
use crate::expression::ValidGrouping;
use crate::query_builder::{AsQuery, SelectStatement};
use crate::query_source::Table;

/// The `having` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `having` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait HavingDsl<Predicate> {
    /// The type returned by `.having`.
    type Output;

    /// See the trait documentation.
    fn having(self, predicate: Predicate) -> dsl::Having<Self, Predicate>;
}

impl<T, Predicate> HavingDsl<Predicate> for T
where
    T: Table + AsQuery<Query = SelectStatement<T>>,
    SelectStatement<T>: HavingDsl<Predicate>,
    T::DefaultSelection: Expression<SqlType = T::SqlType> + ValidGrouping<()>,
    T::SqlType: TypedExpressionType,
{
    type Output = dsl::Having<SelectStatement<T>, Predicate>;

    fn having(self, predicate: Predicate) -> dsl::Having<Self, Predicate> {
        self.as_query().having(predicate)
    }
}
