use expression::*;
use query_builder::*;
use query_dsl::*;
use types::NativeSqlType;

impl<ST, S, F, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<ST, S, F> where
    Selection: Expression,
    SelectStatement<Type, Selection, F>: Query<SqlType=Type>,
    Type: NativeSqlType,
{
    type Output = SelectStatement<Type, Selection, F>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(selection, self.from)
    }
}
