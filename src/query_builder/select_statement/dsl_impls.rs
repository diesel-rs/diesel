use expression::*;
use query_builder::*;
use query_dsl::*;
use types::{Bool, NativeSqlType};

impl<ST, S, F, W, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<ST, S, F, W> where
    Selection: Expression,
    SelectStatement<Type, Selection, F, W>: Query<SqlType=Type>,
    Type: NativeSqlType,
{
    type Output = SelectStatement<Type, Selection, F, W>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(selection, self.from, self.where_clause)
    }
}

impl<ST, S, F, W, Predicate> FilterDsl<Predicate>
    for SelectStatement<ST, S, F, W> where
    Predicate: Expression<SqlType=Bool>,
    SelectStatement<ST, S, F, Predicate>: Query,
{
    type Output = SelectStatement<ST, S, F, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(self.select, self.from, predicate)
    }
}
