use expression::*;
use query_builder::{Query, AsQuery};
use query_source::QuerySource;
use types::NativeSqlType;

pub trait SelectDsl<
    Selection: Expression,
    Type: NativeSqlType = <Selection as Expression>::SqlType,
> {
    type Output: Query<SqlType=Type>;

    fn select(self, selection: Selection) -> Self::Output;
}

impl<T, Selection, Type> SelectDsl<Selection, Type> for T where
    Selection: Expression,
    Type: NativeSqlType,
    T: QuerySource + AsQuery,
    T::Query: SelectDsl<Selection, Type>,
{
    type Output = <T::Query as SelectDsl<Selection, Type>>::Output;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}

pub trait SelectSqlDsl: Sized {
    fn select_sql<A>(self, columns: &str)
        -> <Self as SelectDsl<SqlLiteral<A>>>::Output where
        A: NativeSqlType,
        Self: SelectDsl<SqlLiteral<A>>,
    {
        self.select_sql_inner(columns)
    }

    fn select_sql_inner<A, S>(self, columns: S)
        -> <Self as SelectDsl<SqlLiteral<A>>>::Output where
        A: NativeSqlType,
        S: Into<String>,
        Self: SelectDsl<SqlLiteral<A>>,
    {
        self.select(SqlLiteral::new(columns.into()))
    }
}

impl<T> SelectSqlDsl for T {}
