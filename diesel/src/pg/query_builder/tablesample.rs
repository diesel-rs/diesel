use crate::{
    expression::{Expression, ValidGrouping},
    pg::Pg,
    query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement},
    query_source::QuerySource,
    result::QueryResult,
    sql_types::{Double, SmallInt},
    JoinTo, SelectableExpression, Table,
};

/// Indicates the sampling method for a `TABLESAMPLE method(n)` clause. The provided percentage
/// should be an integer between 0 and 100.
#[derive(Debug, Clone, Copy)]
pub enum TablesampleMethod {
    /// Use the BERNOULLI sampline method. This is row-based, slower but more accurate.
    Bernoulli(i16),

    /// Use the SYSTEM sampling method. This is page-based, faster but less accurate.
    System(i16),
}

/// Indicates the random number seed for a `TABLESAMPLE ... REPEATABLE(f)` clause.
#[derive(Debug, Clone, Copy)]
pub enum TablesampleSeed {
    /// Have PostgreSQL generate an implied random number generator seed.
    Auto,

    /// Provide your own random number generator seed.
    Repeatable(f64),
}

/// Represents a query with a `TABLESAMPLE` clause.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct Tablesample<S> {
    pub source: S,
    pub method: TablesampleMethod,
    pub seed: TablesampleSeed,
}

impl<S> QueryId for Tablesample<S>
where
    S: QueryId,
{
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<S> QuerySource for Tablesample<S>
where
    S: Table + Clone,
    <S as QuerySource>::DefaultSelection: ValidGrouping<()> + SelectableExpression<Tablesample<S>>,
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

impl<S> QueryFragment<Pg> for Tablesample<S>
where
    S: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.source.walk_ast(out.reborrow())?;
        out.push_sql(" TABLESAMPLE ");
        match &self.method {
            TablesampleMethod::Bernoulli(p) => {
                out.push_sql("BERNOULLI(");
                out.push_bind_param::<SmallInt, _>(p)?;
                out.push_sql(")");
            }
            TablesampleMethod::System(p) => {
                out.push_sql("SYSTEM(");
                out.push_bind_param::<SmallInt, _>(p)?;
                out.push_sql(")");
            }
        };
        match &self.seed {
            TablesampleSeed::Auto => { /* no-op, this is the default */ }
            TablesampleSeed::Repeatable(f) => {
                out.push_sql(" REPEATABLE(");
                out.push_bind_param::<Double, _>(f)?;
                out.push_sql(")");
            }
        }
        Ok(())
    }
}

impl<S> AsQuery for Tablesample<S>
where
    S: Table + Clone,
    <S as QuerySource>::DefaultSelection: ValidGrouping<()> + SelectableExpression<Tablesample<S>>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;
    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<S, T> JoinTo<T> for Tablesample<S>
where
    S: JoinTo<T>,
    T: Table,
    S: Table,
{
    type FromClause = <S as JoinTo<T>>::FromClause;
    type OnClause = <S as JoinTo<T>>::OnClause;

    fn join_target(rhs: T) -> (Self::FromClause, Self::OnClause) {
        <S as JoinTo<T>>::join_target(rhs)
    }
}

impl<S> Table for Tablesample<S>
where
    S: Table + Clone + AsQuery,

    <S as Table>::PrimaryKey: SelectableExpression<Tablesample<S>>,
    <S as Table>::AllColumns: SelectableExpression<Tablesample<S>>,
    <S as QuerySource>::DefaultSelection: ValidGrouping<()> + SelectableExpression<Tablesample<S>>,
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
    use crate::pg::Pg;
    use crate::query_builder::QueryBuilder;
    use diesel::dsl::*;
    use diesel::*;

    // TODO: Borrowed from src/pg/transaction.rs -- should this be extracted elsewhere?
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
            users::table.tablesample(TablesampleMethod::Bernoulli(10), TablesampleSeed::Auto),
            "\"users\" TABLESAMPLE BERNOULLI($1)"
        );

        assert_sql!(
            users::table.tablesample(TablesampleMethod::System(10), TablesampleSeed::Auto),
            "\"users\" TABLESAMPLE SYSTEM($1)"
        );

        assert_sql!(
            users::table.tablesample(
                TablesampleMethod::System(10),
                TablesampleSeed::Repeatable(42.0),
            ),
            "\"users\" TABLESAMPLE SYSTEM($1) REPEATABLE($2)"
        );
    }
}
