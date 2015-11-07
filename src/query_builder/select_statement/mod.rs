use expression::*;
use query_source::QuerySource;
use std::marker::PhantomData;
use super::{Query, QueryBuilder, BuildQueryResult};
use types::NativeSqlType;

#[derive(Debug, Clone, Copy)]
pub struct SelectStatement<SqlType, Select, From> {
    select: Select,
    from: From,
    _marker: PhantomData<SqlType>,
}

impl<ST, S, F> SelectStatement<ST, S, F> {
    pub fn new(select: S, from: F) -> Self {
        SelectStatement {
            select: select,
            from: from,
            _marker: PhantomData,
        }
    }

    pub fn simple(select: S, from: F) -> Self {
        SelectStatement::new(select, from)
    }
}

impl<Type, Select, From> Query for SelectStatement<Type, Select, From> where
    Type: NativeSqlType,
    From: QuerySource,
    Select: SelectableExpression<From, Type>,
{
    type SqlType = Type;

    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        self.from.from_clause(out)
    }
}
