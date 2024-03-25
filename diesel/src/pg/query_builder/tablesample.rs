use crate::expression::{Expression, ValidGrouping};
use crate::pg::Pg;
use crate::query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::query_source::QuerySource;
use crate::result::QueryResult;
use crate::sql_types::{Double, SmallInt};
use crate::{JoinTo, SelectableExpression, Table};
use std::marker::PhantomData;

#[doc(hidden)]
pub trait TablesampleMethod: Clone {
    fn method_name_sql() -> &'static str;
}

#[derive(Clone, Copy, Debug)]
/// Used to specify the `BERNOULLI` sampling method.
pub struct BernoulliMethod;

impl TablesampleMethod for BernoulliMethod {
    fn method_name_sql() -> &'static str {
        "BERNOULLI"
    }
}

#[derive(Clone, Copy, Debug)]
/// Used to specify the `SYSTEM` sampling method.
pub struct SystemMethod;

impl TablesampleMethod for SystemMethod {
    fn method_name_sql() -> &'static str {
        "SYSTEM"
    }
}

/// Represents a query with a `TABLESAMPLE` clause.
#[derive(Debug, Clone, Copy)]
pub struct Tablesample<S, TSM>
where
    TSM: TablesampleMethod,
{
    source: S,
    method: PhantomData<TSM>,
    portion: i16,
    seed: Option<f64>,
}

impl<S, TSM> Tablesample<S, TSM>
where
    TSM: TablesampleMethod,
{
    pub(crate) fn new(source: S, portion: i16) -> Tablesample<S, TSM> {
        Tablesample {
            source,
            method: PhantomData,
            portion,
            seed: None,
        }
    }

    /// This method allows you to specify the random number generator seed to use in the sampling
    /// method. This allows you to obtain repeatable results.
    pub fn with_seed(self, seed: f64) -> Tablesample<S, TSM> {
        Tablesample {
            source: self.source,
            method: self.method,
            portion: self.portion,
            seed: Some(seed),
        }
    }
}

impl<S, TSM> QueryId for Tablesample<S, TSM>
where
    S: QueryId,
    TSM: TablesampleMethod,
{
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<S, TSM> QuerySource for Tablesample<S, TSM>
where
    S: Table + Clone,
    TSM: TablesampleMethod,
    <S as QuerySource>::DefaultSelection:
        ValidGrouping<()> + SelectableExpression<Tablesample<S, TSM>>,
{
    type FromClause = Self;
    type DefaultSelection = <S as QuerySource>::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        self.clone()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.source.default_selection()
    }
}

impl<S, TSM> QueryFragment<Pg> for Tablesample<S, TSM>
where
    S: QueryFragment<Pg>,
    TSM: TablesampleMethod,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.source.walk_ast(out.reborrow())?;
        out.push_sql(" TABLESAMPLE ");
        out.push_sql(TSM::method_name_sql());
        out.push_sql("(");
        out.push_bind_param::<SmallInt, _>(&self.portion)?;
        out.push_sql(")");
        if let Some(f) = &self.seed {
            out.push_sql(" REPEATABLE(");
            out.push_bind_param::<Double, _>(f)?;
            out.push_sql(")");
        }
        Ok(())
    }
}

impl<S, TSM> AsQuery for Tablesample<S, TSM>
where
    S: Table + Clone,
    TSM: TablesampleMethod,
    <S as QuerySource>::DefaultSelection:
        ValidGrouping<()> + SelectableExpression<Tablesample<S, TSM>>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;
    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<S, T, TSM> JoinTo<T> for Tablesample<S, TSM>
where
    S: JoinTo<T>,
    T: Table,
    S: Table,
    TSM: TablesampleMethod,
{
    type FromClause = <S as JoinTo<T>>::FromClause;
    type OnClause = <S as JoinTo<T>>::OnClause;

    fn join_target(rhs: T) -> (Self::FromClause, Self::OnClause) {
        <S as JoinTo<T>>::join_target(rhs)
    }
}

impl<S, TSM> Table for Tablesample<S, TSM>
where
    S: Table + Clone + AsQuery,
    TSM: TablesampleMethod,

    <S as Table>::PrimaryKey: SelectableExpression<Tablesample<S, TSM>>,
    <S as Table>::AllColumns: SelectableExpression<Tablesample<S, TSM>>,
    <S as QuerySource>::DefaultSelection:
        ValidGrouping<()> + SelectableExpression<Tablesample<S, TSM>>,
{
    type PrimaryKey = <S as Table>::PrimaryKey;
    type AllColumns = <S as Table>::AllColumns;

    fn primary_key(&self) -> Self::PrimaryKey {
        self.source.primary_key()
    }

    fn all_columns() -> Self::AllColumns {
        S::all_columns()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::Backend;
    use crate::query_builder::QueryBuilder;
    use diesel::dsl::*;
    use diesel::*;

    macro_rules! assert_sql {
        ($query:expr, $sql:expr) => {
            let mut query_builder = <Pg as Backend>::QueryBuilder::default();
            $query.to_sql(&mut query_builder, &Pg).unwrap();
            let sql = query_builder.finish();
            assert_eq!(sql, $sql);
        };
    }

    table! {
        users {
            id -> Integer,
            name -> VarChar,
        }
    }

    #[test]
    fn test_generated_tablesample_sql() {
        assert_sql!(
            users::table.tablesample_bernoulli(10),
            "\"users\" TABLESAMPLE BERNOULLI($1)"
        );

        assert_sql!(
            users::table.tablesample_system(10),
            "\"users\" TABLESAMPLE SYSTEM($1)"
        );

        assert_sql!(
            users::table.tablesample_system(10).with_seed(42.0),
            "\"users\" TABLESAMPLE SYSTEM($1) REPEATABLE($2)"
        );
    }
}
