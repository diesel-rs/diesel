extern crate postgres;

mod joins;
mod select;

use {FindError, Connection};
use self::select::SelectSqlQuerySource;
use std::convert::Into;
use types::{FromSql, NativeSqlType, ToSql};

pub use self::joins::{JoinTo, InnerJoinSource};

pub trait Queriable<ST: NativeSqlType> {
    type Row: FromSql<ST>;

    fn build(row: Self::Row) -> Self;
}

pub trait QuerySource: Sized {
    type SqlType: NativeSqlType;

    fn select_clause(&self) -> String;
    fn from_clause(&self) -> String;

    fn where_clause(&self) -> Option<String> {
        None
    }

    fn bind_params(&self) -> &[&postgres::types::ToSql] {
        &[]
    }

    fn select<C, T>(self, column: C) -> SelectSqlQuerySource<C::SqlType, Self> where
        C: SelectableColumn<T, Self>,
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

pub trait Column<Table> {
    type SqlType: NativeSqlType;

    fn name(&self) -> String;
}

pub trait Table: QuerySource {
    type PrimaryKey: Column<Self>;

    fn name(&self) -> &str;

    fn primary_key(&self) -> Self::PrimaryKey;

    fn find<T, PK>(&self, connection: &Connection, id: &PK) -> Result<T, FindError> where
        T: Queriable<Self::SqlType>,
        PK: ToSql<<Self::PrimaryKey as Column<Self>>::SqlType>,
    {
        let source = FindSource {
            table: self,
            id: [id],
        };
        match connection.query_one(&source) {
            Ok(Some(record)) => Ok(record),
            Ok(None) => Err(FindError::RecordNotFound),
            Err(e) => Err(FindError::Error(e)),
        }
    }

    fn inner_join<T>(self, other: T) -> InnerJoinSource<Self, T> where
        T: Table,
        Self: JoinTo<T>,
    {
        InnerJoinSource::new(self, other)
    }
}

struct FindSource<'a, T: 'a> {
    table: &'a T,
    id: [&'a postgres::types::ToSql; 1],
}

impl<'a, T> QuerySource for FindSource<'a, T> where
    T: Table,
{
    type SqlType = T::SqlType;

    fn select_clause(&self) -> String {
        self.table.select_clause()
    }

    fn from_clause(&self) -> String {
        self.table.from_clause()
    }

    fn where_clause(&self) -> Option<String> {
        Some(format!("{} = $1", self.table.primary_key().name()))
    }

    fn bind_params(&self) -> &[&postgres::types::ToSql] {
        &self.id
    }
}

pub trait SelectableColumn<T, QS: QuerySource>: Column<T> {}

impl<T, C> SelectableColumn<T, T> for C where
    T: Table,
    C: Column<T>,
{}
