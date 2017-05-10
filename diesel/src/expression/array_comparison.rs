use backend::Backend;
use expression::*;
use expression::helper_types::SqlTypeOf;
use query_builder::*;
use result::QueryResult;
use types::Bool;

#[derive(Debug, Copy, Clone)]
pub struct In<T, U> {
    left: T,
    values: U,
}

#[derive(Debug, Copy, Clone)]
pub struct NotIn<T, U> {
    left: T,
    values: U,
}

pub type EqAny<T, U> = In<T, <U as AsInExpression<SqlTypeOf<T>>>::InExpression>;
pub type NeAny<T, U> = NotIn<T, <U as AsInExpression<SqlTypeOf<T>>>::InExpression>;

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

impl<T, U> Expression for In<T, U> where
    T: Expression,
    U: Expression<SqlType=T::SqlType>,
{
    type SqlType = Bool;
}

impl<T, U> Expression for NotIn<T, U> where
    T: Expression,
    U: Expression<SqlType=T::SqlType>,
{
    type SqlType = Bool;
}

impl<T, U> NonAggregate for In<T, U> where
    In<T, U>: Expression,
{
}

impl<T, U> NonAggregate for NotIn<T, U> where
    NotIn<T, U>: Expression,
{
}

impl<T, U, DB> QueryFragment<DB> for In<T, U> where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB> + MaybeEmpty,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        if self.values.is_empty() {
            out.push_sql("1=0");
        } else {
            try!(self.left.to_sql(out));
            out.push_sql(" IN (");
            try!(self.values.to_sql(out));
            out.push_sql(")");
        }
        Ok(())
    }

    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        self.left.walk_ast(pass.reborrow())?;
        self.values.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<T, U, DB> QueryFragment<DB> for NotIn<T, U> where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB> + MaybeEmpty,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        if self.values.is_empty() {
            out.push_sql("1=1");
        } else {
            try!(self.left.to_sql(out));
            out.push_sql(" NOT IN (");
            try!(self.values.to_sql(out));
            out.push_sql(")");
        }
        Ok(())
    }

    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        self.left.walk_ast(pass.reborrow())?;
        self.values.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl_query_id!(In<T, U>);
impl_query_id!(NotIn<T, U>);
impl_selectable_expression!(In<T, U>);
impl_selectable_expression!(NotIn<T, U>);

use std::marker::PhantomData;
use query_builder::{SelectStatement, BoxedSelectStatement};

pub trait AsInExpression<T> {
    type InExpression: MaybeEmpty + Expression<SqlType=T>;

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

pub trait MaybeEmpty {
    fn is_empty(&self) -> bool;
}

impl<ST, S, F, W, O, L, Of, G> AsInExpression<ST>
    for SelectStatement<S, F, W, O, L, Of, G> where
        SelectStatement<S, F, W, O, L, Of, G>: Query<SqlType=ST>,
        Subselect<SelectStatement<S, F, W, O, L, Of>, ST>: Expression,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect { values: self, _sql_type: PhantomData }
    }
}

impl<'a, ST, QS, DB> AsInExpression<ST>
    for BoxedSelectStatement<'a, ST, QS, DB> where
        Subselect<BoxedSelectStatement<'a, ST, QS, DB>, ST>: Expression<SqlType=ST>,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect { values: self, _sql_type: PhantomData }
    }
}

#[derive(Debug)]
pub struct Many<T>(Vec<T>);

impl<T: Expression> Expression for Many<T> {
    type SqlType = T::SqlType;
}

impl<T> MaybeEmpty for Many<T> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T, QS> SelectableExpression<QS> for Many<T> where
    Many<T>: AppearsOnTable<QS>,
    T: SelectableExpression<QS>,
{
}

impl<T, QS> AppearsOnTable<QS> for Many<T> where
    Many<T>: Expression,
    T: AppearsOnTable<QS>,
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

    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        pass.unsafe_to_cache_prepared();
        for value in &self.0 {
            value.walk_ast(pass.reborrow())?;
        }
        Ok(())
    }
}

impl_query_id!(noop: Many<T>);

#[derive(Debug, Copy, Clone)]
pub struct Subselect<T, ST> {
    values: T,
    _sql_type: PhantomData<ST>,
}

impl<T: Query, ST> Expression for Subselect<T, ST> {
    type SqlType = ST;
}

impl<T, ST> MaybeEmpty for Subselect<T, ST> {
    fn is_empty(&self) -> bool {
        false
    }
}

impl<T, ST, QS> SelectableExpression<QS> for Subselect<T, ST> where
    Subselect<T, ST>: AppearsOnTable<QS>,
    T: Query,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for Subselect<T, ST> where
    Subselect<T, ST>: Expression,
    T: Query,
{
}

impl<T, ST, DB> QueryFragment<DB> for Subselect<T, ST> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.values.to_sql(out)
    }

    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        self.values.walk_ast(pass)
    }
}

impl_query_id!(Subselect<T, ST>);
