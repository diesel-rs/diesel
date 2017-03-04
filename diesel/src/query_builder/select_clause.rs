use backend::Backend;
use expression::SelectableExpression;
use query_builder::*;
use query_source::QuerySource;

#[derive(Debug, Clone, Copy)]
pub struct DefaultSelectClause;
#[derive(Debug, Clone, Copy)]
pub struct SelectClause<T>(pub T);

impl_query_id!(DefaultSelectClause);
impl_query_id!(SelectClause<T>);

pub trait SelectClauseExpression<QS> {
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<QS> for SelectClause<T> where
    T: SelectableExpression<QS>,
{
    type SelectClauseSqlType = T::SqlTypeForSelect;
}

impl<QS> SelectClauseExpression<QS> for DefaultSelectClause where
    QS: QuerySource,
{
    type SelectClauseSqlType = <QS::DefaultSelection as SelectableExpression<QS>>::SqlTypeForSelect;
}

pub trait SelectClauseQueryFragment<QS, DB: Backend> {
    fn to_sql(&self, source: &QS, out: &mut DB::QueryBuilder) -> BuildQueryResult;
    fn collect_binds(&self, source: &QS, out: &mut DB::BindCollector) -> QueryResult<()>;
    fn is_safe_to_cache_prepared(&self, source: &QS) -> bool;
}

impl<T, QS, DB> SelectClauseQueryFragment<QS, DB> for SelectClause<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, _: &QS, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.0.to_sql(out)
    }

    fn collect_binds(&self, _: &QS, out: &mut DB::BindCollector) -> QueryResult<()> {
        self.0.collect_binds(out)
    }

    fn is_safe_to_cache_prepared(&self, _: &QS) -> bool {
        self.0.is_safe_to_cache_prepared()
    }
}

impl<QS, DB> SelectClauseQueryFragment<QS, DB> for DefaultSelectClause where
    DB: Backend,
    QS: QuerySource,
    QS::DefaultSelection: QueryFragment<DB>,
{
    fn to_sql(&self, source: &QS, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        source.default_selection().to_sql(out)
    }

    fn collect_binds(&self, source: &QS, out: &mut DB::BindCollector) -> QueryResult<()> {
        source.default_selection().collect_binds(out)
    }

    fn is_safe_to_cache_prepared(&self, source: &QS) -> bool {
        source.default_selection().is_safe_to_cache_prepared()
    }
}
