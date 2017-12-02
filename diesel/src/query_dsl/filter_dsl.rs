use dsl::Filter;
use expression_methods::*;
use query_source::*;

/// The `filter` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `filter` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait FilterDsl<Predicate> {
    type Output;

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
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait FindDsl<PK> {
    type Output;

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
