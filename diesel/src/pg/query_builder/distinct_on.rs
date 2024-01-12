use crate::expression::SelectableExpression;
use crate::pg::Pg;
use crate::query_builder::order_clause::NoOrderClause;
use crate::query_builder::{
    AstPass, FromClause, QueryFragment, QueryId, SelectQuery, SelectStatement,
};
use crate::query_dsl::methods::DistinctOnDsl;
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::result::QueryResult;
use crate::sql_types::SingleValue;
use crate::QuerySource;
use diesel::query_builder::order_clause::OrderClause;

/// Represents `DISTINCT ON (...)`
#[derive(Debug, Clone, Copy, QueryId)]
#[cfg(feature = "postgres_backend")]
pub struct DistinctOnClause<T>(pub(crate) T);

impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for NoOrderClause {}
impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for OrderClause<(T,)> {}
impl<T> ValidOrderingForDistinct<DistinctOnClause<T>> for OrderClause<T> where T: crate::Expression {}

impl<T> ValidOrderingForDistinct<DistinctOnClause<T>>
    for OrderClause<crate::expression::operators::Asc<T>>
where
    T: crate::Expression,
    T::SqlType: SingleValue,
{
}

impl<T> ValidOrderingForDistinct<DistinctOnClause<T>>
    for OrderClause<crate::expression::operators::Desc<T>>
where
    T: crate::Expression,
    T::SqlType: SingleValue,
{
}

macro_rules! valid_ordering {
    // Special-case: for single tuple elements
    // generate plain impls as well:
    (
        @optional_untuple:
        [generics: $($T: ident)*]
        [distinct: $D:ident]
        [order: $O: ty,]
    ) => {
        // nothing if both a single tuple elements
    };
    (
        @optional_untuple:
        [generics: $($T: ident)*]
        [distinct: $D:ident]
        [order: $($O: ty,)*]
    ) => {
        impl<$($T,)*> ValidOrderingForDistinct<DistinctOnClause<$D>>
            for OrderClause<($($O,)*)>
        {}
    };
    (
        @optional_untuple:
        [generics: $($T: ident)*]
        [distinct: $($D:ident)*]
        [order: $O: ty,]
    ) => {
        impl<$($T,)*> ValidOrderingForDistinct<DistinctOnClause<($($D,)*)>>
            for OrderClause<$O>
        {}
    };
    (
        @optional_untuple:
        [generics: $($T: ident)*]
        [distinct: $($D:ident)*]
        [order: $($O: ty,)*]
    ) => {};
    // Special-case: rule out the all ident case if the
    // corresponding flag is set
    // We want to have these impls if
    // the tuple sizes do **not** match
    // therefore we set the flag below
    (@impl_one:
     [allow_plain = false]
     $generics:tt
     $distinct:tt
     $other:tt
     [$($T_:ident, )*]
    ) => {
        /* skip this one */
    };
    (@impl_one:
     [allow_plain = $allow_plain: expr]
     [generics: $($T:ident)*]
     [distinct: $($D:ident)*]
     [other: $($O:ident)*]
     [$($Ty:ty, )*]
    ) => {
        impl<$($T,)*> ValidOrderingForDistinct<DistinctOnClause<($($D, )*)>>
            for OrderClause<($($Ty, )* $($O,)*)>
        {}
        valid_ordering!(@optional_untuple: [generics: $($T)*] [distinct: $($D)*] [order: $($Ty,)* $($O,)*]);
    };
    (
        @perm:
        $allow_plain:tt
        $generics:tt
        $distinct:tt
        $other:tt
        [acc: $([$($acc:tt)*])*]
        $T:ident
        $($rest:tt)*
    ) => {
        valid_ordering! {
            @perm:
            $allow_plain
            $generics
            $distinct
            $other
                [acc:
                 $(
                     [$($acc)* crate::expression::operators::Asc<$T>, ]
                     [$($acc)*     $T  , ]
                     [$($acc)* crate::expression::operators::Desc<$T>, ]
                 )*
                ]
                $($rest)*
        }
    };
    (
        @perm:
        $allow_plain:tt
        $generics:tt
        $distinct:tt
        $other:tt
        [acc: $($Tys:tt)*]
        /* nothing left */
    ) => (
        $(
            valid_ordering! {@impl_one:
                $allow_plain
                $generics
                $distinct
                $other
                $Tys
            }
        )*
    );
    (@skip_distinct_rev: [generics: $($G: ident)*] [other: $($O: ident)*] [acc: $($T: ident)*]) => {
        valid_ordering!(@perm:
                        [allow_plain = true]
                        [generics: $($G)*]
                        [distinct: $($T)*]
                        [other: $($O)* ]
                        [acc: []]
                        $($T)*
        );
    };
    (@skip_distinct_rev: [generics: $($G: ident)*] [other: $($O: ident)*] [acc: $($I: ident)*] $T: ident $($Ts: ident)*) => {
        valid_ordering!(
            @skip_distinct_rev:
            [generics: $($G)*]
            [other: $($O)*]
            [acc: $T $($I)*]
            $($Ts)*
        );
    };
    (@skip_distinct:
     [generics: $($G: ident)*]
     [acc: $($O: ident)*]
     $T: ident
    ) => {};
    (@skip_distinct:
     [generics: $($G: ident)*]
     [acc: $($O: ident)*]
     $T:ident $($Ts: ident)*
    ) => {
        valid_ordering!(@skip_distinct_rev:
            [generics: $($G)*]
            [other: $($O)* $T]
            [acc: ]
            $($Ts)*
        );
        valid_ordering!(@skip_distinct: [generics: $($G)*] [acc: $($O)* $T] $($Ts)*);
    };
    (@skip_order_rev: [generics: $($G: ident)*] [acc: $($T: ident)*]) => {
        valid_ordering!(@perm:
            [allow_plain = true]
            [generics: $($G)*]
            [distinct: $($G)*]
            [other: ]
            [acc: []]
            $($T)*
        );
    };
    (@skip_order_rev: [generics: $($G: ident)*] [acc: $($I: ident)*] $T: ident $($Ts: ident)*) => {
        valid_ordering!(
            @skip_order_rev:
            [generics: $($G)*]
            [acc: $T $($I)*]
            $($Ts)*
        );
    };
    (@skip_order:
     [generics: $($G: ident)*]
     $T: ident
    ) => {};
    (@skip_order:
     [generics: $($G: ident)*]
     $T: ident $($Ts: ident)*
    ) => {
        valid_ordering!(@skip_order_rev: [generics: $($G)*] [acc: ] $($Ts)*);
        valid_ordering!(@skip_order: [generics: $($G)*] $($Ts)*);
    };
    (@reverse_list: [generics: $($G: ident)*] [acc: $($I: ident)*]) => {
        valid_ordering!(@skip_order: [generics: $($G)*] $($I)*);
        valid_ordering!(@skip_distinct: [generics: $($G)*] [acc: ] $($I)*);
    };
    (@reverse_list: [generics: $($G: ident)*] [acc: $($I: ident)*] $T: ident $($Ts: ident)*) => {
        valid_ordering!(@reverse_list: [generics: $($G)*] [acc: $T $($I)*] $($Ts)*);
    };
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            valid_ordering!(@perm:
                [allow_plain = false]
                [generics: $($T)*]
                [distinct: $($T)*]
                [other: ]
                [acc: []]
                $($T)*
            );
            valid_ordering!(@reverse_list: [generics: $($T)*] [acc: ] $($T)*);
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
