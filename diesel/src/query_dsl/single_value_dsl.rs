use super::methods::LimitDsl;
use crate::dsl::Limit;
use crate::expression::grouped::Grouped;
use crate::expression::subselect::Subselect;
use crate::query_builder::SelectQuery;
use crate::sql_types::{IntoNullable, SingleValue};

/// The `single_value` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `single_value` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait SingleValueDsl {
    /// The type returned by `.single_value`.
    type Output;

    /// See the trait documentation.
    fn single_value(self) -> Self::Output;
}

impl<T, ST> SingleValueDsl for T
where
    Self: SelectQuery<SqlType = ST> + LimitDsl,
    ST: IntoNullable,
    ST::Nullable: SingleValue,
{
    type Output = Grouped<Subselect<Limit<Self>, ST::Nullable>>;

    fn single_value(self) -> Self::Output {
        Grouped(Subselect::new(self.limit(1)))
    }
}
