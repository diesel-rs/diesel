use types::{FromSql, NativeSqlType};
use std::marker::PhantomData;
use std::convert::Into;

pub trait Queriable<ST: NativeSqlType> {
    type Row: FromSql<ST>;

    fn build(row: Self::Row) -> Self;
}

pub trait QuerySource: Sized {
    type SqlType: NativeSqlType;

    fn select_clause(&self) -> String;
    fn from_clause(&self) -> String;

    fn select_sql<A: NativeSqlType>(self, columns: &str)
        -> SelectSqlQuerySource<A, Self>
    {
        self.select_sql_inner(columns)
    }

    fn select_sql_inner<A, S>(self, columns: S)
        -> SelectSqlQuerySource<A, Self> where
        A: NativeSqlType,
        S: Into<String>
    {
        SelectSqlQuerySource {
            columns: columns.into(),
            source: self,
            _marker: PhantomData,
        }
    }
}

pub trait Column<A: NativeSqlType, T: Table> {
    fn name(&self) -> String;
}

pub trait Table: QuerySource {
    fn name(&self) -> &str;

    fn select<A, C>(self, column: C) -> SelectSqlQuerySource<A, Self> where
        A: NativeSqlType,
        C: Column<A, Self>,
    {
        self.select_sql_inner(column.name())
    }
}

pub struct SelectSqlQuerySource<A, S> where
    A: NativeSqlType,
    S: QuerySource,
{
    columns: String,
    source: S,
    _marker: PhantomData<A>,
}

impl<A, S> QuerySource for SelectSqlQuerySource<A, S> where
    A: NativeSqlType,
    S: QuerySource,
{
    type SqlType = A;

    fn select_clause(&self) -> String {
        self.columns.clone()
    }

    fn from_clause(&self) -> String {
        self.source.from_clause()
    }
}
