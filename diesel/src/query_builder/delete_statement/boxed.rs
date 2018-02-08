use backend::Backend;
use expression::{AppearsOnTable, SelectableExpression};
use query_builder::returning_clause::*;
use query_builder::*;
use query_dsl::RunQueryDsl;
use query_dsl::methods::FilterDsl;
use query_source::Table;
use result::QueryResult;

#[allow(missing_debug_implementations)] // We can't...
/// Represents a boxed SQL `DELETE` statement.
///
/// The where clause of this delete statement has been boxed,
/// allowing `.filter` to be called conditionally without changing
/// the return type.
///
/// The parameters on this struct represent:
///
/// - `'a`: The lifetime of the where clause.
/// - `DB`: The backend this query will be run against.
/// - `T`: The table we are deleting from.
/// - `Ret`: The `RETURNING` clause of this query. The exact types used to
///   represent this are private. You can safely rely on the default type
///   representing the lack of a `RETURNING` clause.
pub struct BoxedDeleteStatement<'a, DB, T, Ret = NoReturningClause> {
    table: T,
    where_clause: Option<Box<QueryFragment<DB> + 'a>>,
    returning: Ret,
}

impl<'a, DB, T, Ret> BoxedDeleteStatement<'a, DB, T, Ret> {
    pub(crate) fn new(
        table: T,
        where_clause: Option<Box<QueryFragment<DB> + 'a>>,
        returning: Ret,
    ) -> Self {
        BoxedDeleteStatement {
            table,
            where_clause,
            returning,
        }
    }

    /// Adds the given predicate to the `WHERE` clause of the statement being
    /// constructed.
    ///
    /// If there is already a `WHERE` clause, the predicate will be appended
    /// with `AND`.
    ///
    /// See [`DeleteStatement::filter`] for examples.
    ///
    /// [`DeleteStatement::filter`]: struct.DeleteStatement.html#method.filter
    pub fn filter<Predicate>(self, predicate: Predicate) -> Self
    where
        Self: FilterDsl<Predicate, Output = Self>,
    {
        FilterDsl::filter(self, predicate)
    }
}

impl<'a, DB, T, Ret, Predicate> FilterDsl<Predicate> for BoxedDeleteStatement<'a, DB, T, Ret>
where
    DB: Backend + 'a,
    Predicate: AppearsOnTable<T> + QueryFragment<DB> + 'a,
{
    type Output = Self;

    fn filter(mut self, predicate: Predicate) -> Self::Output {
        use expression::operators::And;
        self.where_clause = match self.where_clause {
            Some(where_clause) => Some(Box::new(And::new(where_clause, predicate))),
            None => Some(Box::new(predicate)),
        };
        self
    }
}

impl<'a, DB, T, Ret> QueryFragment<DB> for BoxedDeleteStatement<'a, DB, T, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("DELETE FROM ");
        self.table.from_clause().walk_ast(out.reborrow())?;
        if let Some(ref where_clause) = self.where_clause {
            out.push_sql(" WHERE ");
            where_clause.walk_ast(out.reborrow())?;
        }
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a, DB, T, Ret> QueryId for BoxedDeleteStatement<'a, DB, T, Ret> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, T, DB> AsQuery for BoxedDeleteStatement<'a, DB, T>
where
    T: Table,
    T::AllColumns: SelectableExpression<T>,
    BoxedDeleteStatement<'a, DB, T, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = BoxedDeleteStatement<'a, DB, T, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<'a, DB, T, Ret> Query for BoxedDeleteStatement<'a, DB, T, ReturningClause<Ret>>
where
    T: Table,
    Ret: SelectableExpression<T>,
{
    type SqlType = Ret::SqlType;
}

impl<'a, T, DB, Ret, Conn> RunQueryDsl<Conn> for BoxedDeleteStatement<'a, DB, T, Ret> {}

impl<'a, DB, T> BoxedDeleteStatement<'a, DB, T> {
    /// Specify what expression is returned after execution of the `delete`.
    ///
    /// See [`DeleteStatement::returning`] for examples.
    ///
    /// [`DeleteStatement::returning`]: struct.DeleteStatement.html#method.returning
    pub fn returning<E>(self, returns: E) -> BoxedDeleteStatement<'a, DB, T, ReturningClause<E>>
    where
        E: SelectableExpression<T>,
        BoxedDeleteStatement<'a, DB, T, ReturningClause<E>>: Query,
    {
        BoxedDeleteStatement::new(self.table, self.where_clause, ReturningClause(returns))
    }
}
