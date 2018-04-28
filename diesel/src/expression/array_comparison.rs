use backend::Backend;
use expression::subselect::Subselect;
use expression::*;
use query_builder::*;
use result::QueryResult;
use sql_types::Bool;

#[derive(Debug, Copy, Clone, QueryId)]
pub struct In<T, U> {
    left: T,
    values: U,
}

#[derive(Debug, Copy, Clone, QueryId)]
pub struct NotIn<T, U> {
    left: T,
    values: U,
}

impl<T, U> In<T, U> {
    pub fn new(left: T, values: U) -> Self {
        In {
            left: left,
            values: values,
        }
    }
}

impl<T, U> NotIn<T, U> {
    pub fn new(left: T, values: U) -> Self {
        NotIn {
            left: left,
            values: values,
        }
    }
}

impl<T, U> Expression for In<T, U>
where
    T: Expression,
    U: Expression<SqlType = T::SqlType>,
{
    type SqlType = Bool;
}

impl<T, U> Expression for NotIn<T, U>
where
    T: Expression,
    U: Expression<SqlType = T::SqlType>,
{
    type SqlType = Bool;
}

impl<T, U> NonAggregate for In<T, U>
where
    In<T, U>: Expression,
{
}

impl<T, U> NonAggregate for NotIn<T, U>
where
    NotIn<T, U>: Expression,
{
}

impl<T, U, DB> QueryFragment<DB> for In<T, U>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB> + MaybeEmpty,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if self.values.is_empty() {
            out.push_sql("1=0");
        } else {
            self.left.walk_ast(out.reborrow())?;
            out.push_sql(" IN (");
            self.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

impl<T, U, DB> QueryFragment<DB> for NotIn<T, U>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB> + MaybeEmpty,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if self.values.is_empty() {
            out.push_sql("1=1");
        } else {
            self.left.walk_ast(out.reborrow())?;
            out.push_sql(" NOT IN (");
            self.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

impl_selectable_expression!(In<T, U>);
impl_selectable_expression!(NotIn<T, U>);

use query_builder::{BoxedSelectStatement, SelectStatement};

pub trait AsInExpression<T> {
    type InExpression: MaybeEmpty + Expression<SqlType = T>;

    fn as_in_expression(self) -> Self::InExpression;
}

impl<I, T, ST> AsInExpression<ST> for I
where
    I: IntoIterator<Item = T>,
    T: AsExpression<ST>,
{
    type InExpression = Many<T::Expression>;

    fn as_in_expression(self) -> Self::InExpression {
        let expressions = self.into_iter().map(AsExpression::as_expression).collect();
        Many(expressions)
    }
}

pub trait MaybeEmpty {
    fn is_empty(&self) -> bool;
}

use query_builder::select_clause::NotNullableSelectClause;
use sql_types::{NotNull, Nullable};
use query_builder::select_clause::NullableSelectClause;

impl<ST, S, F, W, O, L, Of, G, FU> AsInExpression<ST> for SelectStatement<F, S, W, O, L, Of, G, FU>
where
    Subselect<Self, ST>: Expression<SqlType = ST>,
    Self: SelectQuery<SqlType = ST>,
    S: NotNullableSelectClause,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

impl<ST, S, F, W, O, L, Of, G, FU> AsInExpression<Nullable<ST>>
    for SelectStatement<F, NullableSelectClause<S>, W, O, L, Of, G, FU>
where
    Subselect<Self, Nullable<ST>>: Expression<SqlType = Nullable<ST>>,
    Self: SelectQuery<SqlType = Nullable<ST>>,
    ST: NotNull,
{
    type InExpression = Subselect<Self, Nullable<ST>>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

impl<'a, ST, QS, DB> AsInExpression<ST> for BoxedSelectStatement<'a, ST, QS, DB>
where
    Subselect<BoxedSelectStatement<'a, ST, QS, DB>, ST>: Expression<SqlType = ST>,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct Many<T>(Vec<T>);

impl<T: Expression> Expression for Many<T> {
    type SqlType = T::SqlType;
}

impl<T> MaybeEmpty for Many<T> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T, QS> SelectableExpression<QS> for Many<T>
where
    Many<T>: AppearsOnTable<QS>,
    T: SelectableExpression<QS>,
{
}

impl<T, QS> AppearsOnTable<QS> for Many<T>
where
    Many<T>: Expression,
    T: AppearsOnTable<QS>,
{
}

impl<T, DB> QueryFragment<DB> for Many<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        let mut first = true;
        for value in &self.0 {
            if first {
                first = false;
            } else {
                out.push_sql(", ");
            }
            value.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<T> QueryId for Many<T> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
