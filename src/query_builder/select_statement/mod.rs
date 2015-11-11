mod dsl_impls;

use expression::*;
use query_source::QuerySource;
use std::marker::PhantomData;
use super::{Query, QueryBuilder, QueryFragment, BuildQueryResult};
use super::where_clause::NoWhereClause;
use types::NativeSqlType;

#[derive(Debug, Clone, Copy)]
pub struct SelectStatement<SqlType, Select, From, Where = NoWhereClause> {
    select: Select,
    from: From,
    where_clause: Where,
    _marker: PhantomData<SqlType>,
}

impl<ST, S, F, W> SelectStatement<ST, S, F, W> {
    pub fn new(select: S, from: F, where_clause: W) -> Self {
        SelectStatement {
            select: select,
            from: from,
            where_clause: where_clause,
            _marker: PhantomData,
        }
    }
}

impl<ST, S, F> SelectStatement<ST, S, F> {
    pub fn simple(select: S, from: F) -> Self {
        SelectStatement::new(select, from, NoWhereClause)
    }
}

impl<Type, Select, From, Where> Query for SelectStatement<Type, Select, From, Where> where
    Type: NativeSqlType,
    SelectStatement<Type, Select, From, Where>: QueryFragment
{
    type SqlType = Type;
}

impl<Type, Select, From, Where> QueryFragment for SelectStatement<Type, Select, From, Where> where
    Type: NativeSqlType,
    From: QuerySource,
    Select: SelectableExpression<From, Type>,
    Where: QueryFragment,
{
    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause(out));
        self.where_clause.to_sql(out)
    }
}
