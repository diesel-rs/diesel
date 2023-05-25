use crate::expression::SelectableExpression;
use crate::pg::Pg;
use crate::query_builder::order_clause::NoOrderClause;
use crate::query_builder::{
    AstPass, FromClause, QueryFragment, QueryId, SelectQuery, SelectStatement,
};
use crate::query_dsl::methods::DistinctOnDsl;
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::result::QueryResult;
use crate::QuerySource;
use diesel::query_builder::order_clause::OrderClause;

/// Represents `DISTINCT ON (...)`
#[derive(Debug, Clone, Copy, QueryId)]
#[cfg(feature = "postgres_backend")]
pub struct DistinctOnClause<T>(pub(crate) T);

impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for NoOrderClause {}
impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for OrderClause<(T,)> {}
impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for OrderClause<T> where T: crate::Column {}
impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for OrderClause<crate::helper_types::Desc<T>> where
    T: crate::Column
{
}
impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for OrderClause<crate::helper_types::Asc<T>> where
    T: crate::Column
{
}
macro_rules! valid_ordering {
    (@skip: ($ST1: ident, $($ST:ident,)*), $T1:ident, ) => {};
    (@skip: ($ST1: ident, $($ST:ident,)*), $T1:ident, $($T:ident,)+) => {
        valid_ordering!(($($ST,)*), ($ST1,), $($T,)*);
    };
    (($ST1: ident,), ($($OT:ident,)*), $T1:ident,) => {
        #[allow(unused_parens)]
        impl<$T1, $ST1, $($OT,)*> ValidOrderingForDistinct<DistinctOnClause<($ST1, $($OT,)*)>> for OrderClause<($T1)>
        where $T1: crate::pg::OrderDecorator<Column = $ST1>,
        {}
        impl<$T1, $ST1, $($OT,)*> ValidOrderingForDistinct<DistinctOnClause<($ST1, $($OT,)*)>> for OrderClause<($T1,)>
        where $T1: crate::pg::OrderDecorator<Column = $ST1>,
        {}
        impl<$T1, $ST1, $($OT,)*> ValidOrderingForDistinct<DistinctOnClause<($T1,)>> for OrderClause<($ST1, $($OT,)*)>
        where $ST1: crate::pg::OrderDecorator<Column = $T1>,
        {}

        impl<$T1, $ST1, $($OT,)*> ValidOrderingForDistinct<DistinctOnClause<$T1>> for OrderClause<($ST1, $($OT,)*)>
        where $ST1: crate::pg::OrderDecorator<Column = $T1>,
              $T1: crate::Column,
        {}
    };
    (($ST1: ident, $($ST:ident,)*), ($($OT: ident,)*), $T1:ident, $($T:ident,)+) => {
        impl<$T1, $($T,)* $ST1, $($ST,)* $($OT,)*> ValidOrderingForDistinct<DistinctOnClause<($ST1, $($ST,)* $($OT,)*)>> for OrderClause<($T1, $($T,)*)>
        where $T1: crate::pg::OrderDecorator<Column = $ST1>,
              $($T: crate::pg::OrderDecorator<Column = $ST>,)*
        {}
        impl<$T1, $($T,)* $ST1, $($ST,)* $($OT,)*> ValidOrderingForDistinct<DistinctOnClause<($T1, $($T,)*)>> for OrderClause<($ST1, $($ST,)* $($OT,)*)>
        where $ST1: crate::pg::OrderDecorator<Column = $T1>,
              $($ST: crate::pg::OrderDecorator<Column = $T>,)*
        {}
        valid_ordering!(($($ST,)*), ($($OT,)* $ST1,), $($T,)*);
    };
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<$($T,)* $($ST,)*> ValidOrderingForDistinct<DistinctOnClause<($($T,)*)>> for OrderClause<($($ST,)*)>
            where $($ST: crate::pg::OrderDecorator<Column = $T>,)*
            {}
            valid_ordering!(@skip: ($($ST,)*), $($T,)*);
        )*
    }
}

// we only generate these impl up to a tuple size of 5 as we generate n*n + 4 impls here
// If we would generate these impls up to max_table_column_count tuple elements that
// would be a really large number for 128 tuple elements (~64k trait impls)
// It's fine to increase this number at some point in the future gradually
diesel_derives::__diesel_for_each_tuple!(valid_ordering, 5);

/// A decorator trait for `OrderClause`
/// It helps to have bounds on either Col, Asc<Col> and Desc<Col>.
pub trait OrderDecorator {
    /// A column on a database table.
    type Column;
}

impl<C> OrderDecorator for C
where
    C: crate::Column,
{
    type Column = C;
}

impl<C> OrderDecorator for crate::helper_types::Asc<C> {
    type Column = C;
}

impl<C> OrderDecorator for crate::helper_types::Desc<C> {
    type Column = C;
}

impl<T> QueryFragment<Pg> for DistinctOnClause<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("DISTINCT ON (");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<ST, F, S, D, W, O, LOf, G, H, Selection> DistinctOnDsl<Selection>
    for SelectStatement<FromClause<F>, S, D, W, O, LOf, G, H>
where
    F: QuerySource,
    Selection: SelectableExpression<F>,
    Self: SelectQuery<SqlType = ST>,
    O: ValidOrderingForDistinct<DistinctOnClause<Selection>>,
    SelectStatement<FromClause<F>, S, DistinctOnClause<Selection>, W, O, LOf, G, H>:
        SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<FromClause<F>, S, DistinctOnClause<Selection>, W, O, LOf, G, H>;

    fn distinct_on(self, selection: Selection) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            DistinctOnClause(selection),
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.having,
            self.locking,
        )
    }
}
