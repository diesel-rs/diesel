mod filter;
mod joins;
mod select;

use expression::{Expression, SelectableExpression, NonAggregate, SqlLiteral};
use expression::count::*;
pub use self::filter::FilteredQuerySource;
pub use self::joins::{InnerJoinSource, LeftOuterJoinSource};
pub use self::select::SelectSqlQuerySource;
use std::convert::Into;
use types::{self, FromSqlRow, NativeSqlType};

pub use self::joins::JoinTo;

pub trait Queriable<ST: NativeSqlType> {
    type Row: FromSqlRow<ST>;

    fn build(row: Self::Row) -> Self;
}

pub trait QuerySource: Sized {
    type SqlType: NativeSqlType;

    fn select_clause(&self) -> String;
    fn from_clause(&self) -> String;
    fn where_clause(&self) -> Option<(String, Vec<Option<Vec<u8>>>)>;

    fn select<E, ST>(self, expr: E) -> SelectSqlQuerySource<ST, Self, E> where
        SelectSqlQuerySource<ST, Self, E>: QuerySource,
    {
        SelectSqlQuerySource::new(expr, self)
    }

    fn count(self) -> SelectSqlQuerySource<types::BigInt, Self, CountStar> {
        self.select(count_star())
    }

    fn select_sql<A: NativeSqlType>(self, columns: &str)
        -> SelectSqlQuerySource<A, Self, SqlLiteral<A>>
    {
        self.select_sql_inner(columns)
    }

    fn select_sql_inner<A, S>(self, columns: S)
        -> SelectSqlQuerySource<A, Self, SqlLiteral<A>> where
        A: NativeSqlType,
        S: Into<String>
    {
        let sql = SqlLiteral::new(columns.into());
        SelectSqlQuerySource::new(sql, self)
    }

    fn filter<T>(self, predicate: T) -> FilteredQuerySource<Self, T> where
        T: SelectableExpression<Self, types::Bool>,
    {
        FilteredQuerySource::new(self, predicate)
    }
}

pub trait Column {
    type Table: Table;
    type SqlType: NativeSqlType;

    fn name(&self) -> String;

    fn qualified_name(&self) -> String;
}

impl<C: Column> Expression for C {
    type SqlType = <Self as Column>::SqlType;

    fn to_sql(&self) -> String {
        self.qualified_name()
    }
}

impl<C: Column> SelectableExpression<C::Table> for C {
}

impl<C: Column> NonAggregate for C {
}

pub trait Table: QuerySource {
    type PrimaryKey: Column<Table=Self>;
    fn name(&self) -> &str;
    fn primary_key(&self) -> Self::PrimaryKey;

    fn inner_join<T>(self, other: T) -> InnerJoinSource<Self, T> where
        T: Table,
        Self: JoinTo<T>,
    {
        InnerJoinSource::new(self, other)
    }

    fn left_outer_join<T>(self, other: T) -> LeftOuterJoinSource<Self, T> where
        T: Table,
        Self: JoinTo<T>,
    {
        LeftOuterJoinSource::new(self, other)
    }
}
