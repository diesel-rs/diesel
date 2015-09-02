mod joins;
mod select;

use types::{FromSql, NativeSqlType};
use std::convert::Into;
pub use self::joins::InnerJoinSource;
use self::select::SelectSqlQuerySource;

pub use self::joins::JoinTo;

pub trait Queriable<ST: NativeSqlType> {
    type Row: FromSql<ST>;

    fn build(row: Self::Row) -> Self;
}

pub trait QuerySource: Sized {
    type SqlType: NativeSqlType;

    fn select_clause(&self) -> String;
    fn from_clause(&self) -> String;

    fn select<A, C, T>(self, column: C) -> SelectSqlQuerySource<A, Self> where
        A: NativeSqlType,
        C: SelectableColumn<A, T, Self>,
    {
        self.select_sql_inner(column.name())
    }

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
        SelectSqlQuerySource::new(columns.into(), self)
    }
}

pub trait Column<A: NativeSqlType, T> {
    fn name(&self) -> String;
}

pub trait Table: QuerySource {
    fn name(&self) -> &str;

    fn inner_join<T>(self, other: T) -> InnerJoinSource<Self, T> where
        T: Table,
        Self: JoinTo<T>,
    {
        InnerJoinSource::new(self, other)
    }
}

pub trait SelectableColumn<A, T, QS: QuerySource>: Column<A, T> where
    A: NativeSqlType,
{}

impl<A, T, C> SelectableColumn<A, T, T> for C where
    A: NativeSqlType,
    T: Table,
    C: Column<A, T>,
{}
