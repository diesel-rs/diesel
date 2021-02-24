use super::methods::LimitDsl;
use crate::dsl::Limit;
use crate::expression::grouped::Grouped;
use crate::expression::subselect::Subselect;
use crate::query_builder::SelectQuery;
use crate::sql_types::IntoNullable;

/// The `single_value` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `single_value` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait SingleValueDsl {
    /// The type returned by `.single_value`.
    type Output;

    /// See the trait documentation.
    fn single_value(self) -> Self::Output;
}

impl<T> SingleValueDsl for T
where
    Self: SelectQuery + LimitDsl,
    <Self as SelectQuery>::SqlType: IntoNullable,
{
    type Output =
        Grouped<Subselect<Limit<Self>, <<Self as SelectQuery>::SqlType as IntoNullable>::Nullable>>;

    fn single_value(self) -> Self::Output {
        Grouped(Subselect::new(self.limit(1)))
    }
}
