use backend::Backend;
use expression::{Expression, SelectableExpression};
use query_builder::*;
use query_source::QuerySource;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct SelectClause<T>(pub T);

pub trait SelectClauseExpression<QS> {
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectClause<T>
where
    T: SelectableExpression<QS>,
{
    type SelectClauseSqlType = T::SqlType;
}

impl<QS> SelectClauseExpression<QS> for DefaultSelectClause
where
    QS: QuerySource,
{
    type SelectClauseSqlType = <QS::DefaultSelection as Expression>::SqlType;
}

pub trait SelectClauseQueryFragment<QS, DB: Backend> {
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()>;
}

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for SelectClause<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, _: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<QS, DB> SelectClauseQueryFragment<QS, DB> for DefaultSelectClause
where
    DB: Backend,
    QS: QuerySource,
    QS::DefaultSelection: QueryFragment<DB>,
{
    fn walk_ast(&self, source: &QS, pass: AstPass<DB>) -> QueryResult<()> {
        source.default_selection().walk_ast(pass)
    }
}

macro_rules! named_query_fragment {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(

            impl<$($T,)*> NamedQueryFragment for SelectClause<($($T,)*)>
            where
                $($T: NamedQueryFragment,)*
            {
                type Name = ($($T::Name,)*);
            }

        )*
    }
}

__diesel_for_each_tuple!(named_query_fragment);

use query_source::Column;

impl<C> NamedQueryFragment for SelectClause<C>
where
    C: Column + NamedQueryFragment,
{
    type Name = (C::Name,);
}
