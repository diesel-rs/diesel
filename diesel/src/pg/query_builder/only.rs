use crate::expression::{Expression, ValidGrouping};
use crate::pg::Pg;
use crate::query_builder::{AsQuery, AstPass, FromClause, QueryFragment, QueryId, SelectStatement};
use crate::query_source::QuerySource;
use crate::result::QueryResult;
use crate::{JoinTo, SelectableExpression, Table};

/// Represents a query with an `ONLY` clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct Only<S> {
    pub(crate) source: S,
}

impl<S> QueryId for Only<S>
where
    Self: 'static,
    S: QueryId,
{
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = <S as QueryId>::HAS_STATIC_QUERY_ID;
}

impl<S> QuerySource for Only<S>
where
    S: Table + Clone,
    <S as QuerySource>::DefaultSelection: ValidGrouping<()> + SelectableExpression<Only<S>>,
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

impl<S> QueryFragment<Pg> for Only<S>
where
    S: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        pass.push_sql(" ONLY ");
        self.source.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<S> AsQuery for Only<S>
where
    S: Table + Clone,
    <S as QuerySource>::DefaultSelection: ValidGrouping<()> + SelectableExpression<Only<S>>,
{
    type SqlType = <<Self as QuerySource>::DefaultSelection as Expression>::SqlType;
    type Query = SelectStatement<FromClause<Self>>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(self)
    }
}

impl<S, T> JoinTo<T> for Only<S>
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
impl<S> Table for Only<S>
where
    S: Table + Clone + AsQuery,

    <S as Table>::PrimaryKey: SelectableExpression<Only<S>>,
    <S as Table>::AllColumns: SelectableExpression<Only<S>>,
    <S as QuerySource>::DefaultSelection: ValidGrouping<()> + SelectableExpression<Only<S>>,
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
