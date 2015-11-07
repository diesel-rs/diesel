use QuerySource;
use expression::SelectableExpression;
use query_builder::{QueryBuilder, BuildQueryResult};
use std::marker::PhantomData;
use types::NativeSqlType;

#[derive(Copy, Clone)]
pub struct SelectSqlQuerySource<A, S, E> {
    columns: E,
    source: S,
    _marker: PhantomData<A>,
}

impl<A, S, E> SelectSqlQuerySource<A, S, E> {
    pub fn new(columns: E, source: S) -> Self {
        SelectSqlQuerySource {
            columns: columns,
            source: source,
            _marker: PhantomData,
        }
    }
}


impl<A, S, E> QuerySource for SelectSqlQuerySource<A, S, E> where
    A: NativeSqlType,
    S: QuerySource,
    E: SelectableExpression<S, A>,
{
    type SqlType = A;

    fn select_clause(&self) -> String {
        self.columns.to_sql()
    }

    fn from_clause(&self) -> String {
        self.source.from_clause()
    }

    fn where_clause(&self) -> Option<(String, Vec<Option<Vec<u8>>>)> {
        self.source.where_clause()
    }

    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql("SELECT ");
        out.push_sql(&self.select_clause());
        out.push_sql(" FROM ");
        out.push_sql(&self.from_clause());
        if let Some((sql, mut binds)) = self.where_clause() {
            out.push_sql(" WHERE ");
            out.push_sql(&sql);
            out.push_binds(&mut binds);
        }
        Ok(())
    }
}
