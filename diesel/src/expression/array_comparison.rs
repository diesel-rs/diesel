use backend::Backend;
use expression::*;
use query_builder::{QueryBuilder, QueryFragment, BuildQueryResult};
use types::Bool;

pub struct In<T, U> {
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

impl<T, U> Expression for In<T, U> where
    T: Expression,
    U: Expression<SqlType=T::SqlType>,
{
    type SqlType = Bool;
}

impl<T, U, QS> SelectableExpression<QS> for In<T, U> where
    In<T, U>: Expression,
    T: SelectableExpression<QS>,
    U: SelectableExpression<QS>,
{
}

impl<T, U> NonAggregate for In<T, U> where
    In<T, U>: Expression,
{
}

impl<T, U, DB> QueryFragment<DB> for In<T, U> where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.left.to_sql(out));
        out.push_sql(" IN (");
        try!(self.values.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

use std::marker::PhantomData;
use query_builder::SelectStatement;

pub trait AsInExpression<T> {
    type InExpression: Expression<SqlType=T>;

    fn as_in_expression(self) -> Self::InExpression;
}

impl<I, T, ST> AsInExpression<ST> for I where
    I: IntoIterator<Item=T>,
    T: AsExpression<ST>,
{
    type InExpression = Many<T::Expression>;

    fn as_in_expression(self) -> Self::InExpression {
        let expressions = self.into_iter()
            .map(AsExpression::as_expression).collect();
        Many(expressions)
    }
}

impl<ST, S, F, W, O, L, Of> AsInExpression<ST>
    for SelectStatement<ST, S, F, W, O, L, Of> where
        SelectStatement<ST, S, F, W, O, L, Of>: Expression,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect { values: self, _sql_type: PhantomData }
    }
}

pub struct Many<T>(Vec<T>);

impl<T: Expression> Expression for Many<T> {
    type SqlType = T::SqlType;
}

impl<T, QS> SelectableExpression<QS> for Many<T> where
    Many<T>: Expression,
    T: SelectableExpression<QS>,
{
}

impl<T, DB> QueryFragment<DB> for Many<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.0[0].to_sql(out));
        for value in self.0[1..].iter() {
            out.push_sql(", ");
            try!(value.to_sql(out));
        }
        Ok(())
    }
}

pub struct Subselect<T, ST> {
    values: T,
    _sql_type: PhantomData<ST>,
}

impl<T: Expression, ST> Expression for Subselect<T, ST> {
    type SqlType = ST;
}

impl<T, ST, QS> SelectableExpression<QS> for Subselect<T, ST> where
    Subselect<T, ST>: Expression,
    T: SelectableExpression<QS>,
{
}

impl<T, ST, DB> QueryFragment<DB> for Subselect<T, ST> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.values.to_sql(out)
    }
}
