use crate::dsl::{Filter, OrFilter};
use crate::expression_methods::*;
use crate::query_source::*;

/// The `filter` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `filter` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait FilterDsl<Predicate> {
    /// The type returned by `.filter`.
    type Output;

    /// See the trait documentation.
    fn filter(self, predicate: Predicate) -> Self::Output;
}

impl<T, Predicate> FilterDsl<Predicate> for T
where
    T: Table,
    T::Query: FilterDsl<Predicate>,
{
    type Output = Filter<T::Query, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}

/// The `find` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `find` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait FindDsl<PK> {
    /// The type returned by `.find`.
    type Output;

    /// See the trait documentation.
    fn find(self, id: PK) -> Self::Output;
}

impl<T, PK> FindDsl<PK> for T
where
    T: Table + FilterDsl<<<T as Table>::PrimaryKey as EqAll<PK>>::Output>,
    T::PrimaryKey: EqAll<PK>,
{
    type Output = Filter<Self, <T::PrimaryKey as EqAll<PK>>::Output>;

    fn find(self, id: PK) -> Self::Output {
        let primary_key = self.primary_key();
        self.filter(primary_key.eq_all(id))
    }
}

/// The `or_filter` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `or_filter` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait OrFilterDsl<Predicate> {
    /// The type returned by `.filter`.
    type Output;

    /// See the trait documentation.
    fn or_filter(self, predicate: Predicate) -> Self::Output;
}

impl<T, Predicate> OrFilterDsl<Predicate> for T
where
    T: Table,
    T::Query: OrFilterDsl<Predicate>,
{
    type Output = OrFilter<T::Query, Predicate>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().or_filter(predicate)
    }
}
