use expression::*;
use query_builder::{Query, AsQuery};
use query_source::QuerySource;

/// Sets the select clause of a query. If there was already a select clause, it
/// will be overridden. The expression passed to `select` must actually be valid
/// for the query (only contains columns from the target table, doesn't mix
/// aggregate + non-aggregate expressions, etc).
pub trait SelectDsl<
    Selection: Expression,
    Type = <Selection as Expression>::SqlType,
> {
    type Output: Query<SqlType=Type>;

    fn select(self, selection: Selection) -> Self::Output;
}

impl<T, Selection, Type> SelectDsl<Selection, Type> for T where
    Selection: Expression,
    T: QuerySource + AsQuery,
    T::Query: SelectDsl<Selection, Type>,
{
    type Output = <T::Query as SelectDsl<Selection, Type>>::Output;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}

#[doc(hidden)]
pub trait SelectSqlDsl: Sized {
    fn select_sql<A>(self, columns: &str)
        -> <Self as SelectDsl<SqlLiteral<A>>>::Output where
        Self: SelectDsl<SqlLiteral<A>>,
    {
        self.select_sql_inner(columns)
    }

    fn select_sql_inner<A, S>(self, columns: S)
        -> <Self as SelectDsl<SqlLiteral<A>>>::Output where
        S: Into<String>,
        Self: SelectDsl<SqlLiteral<A>>,
    {
        self.select(SqlLiteral::new(columns.into()))
    }
}

impl<T> SelectSqlDsl for T {}
