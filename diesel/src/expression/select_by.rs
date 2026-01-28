use crate::backend::{Backend, DieselReserveSpecialization};
use crate::dsl::SqlTypeOf;
use crate::expression::{
    AppearsOnTable, Expression, QueryMetadata, Selectable, SelectableExpression,
    TypedExpressionType, ValidGrouping,
};
use crate::query_builder::*;
use crate::result::QueryResult;

#[derive(Debug)]
pub struct SelectBy<T: Selectable<DB>, DB: Backend> {
    selection: T::SelectExpression,
    p: std::marker::PhantomData<(T, DB)>,
}

impl<T, DB> Clone for SelectBy<T, DB>
where
    DB: Backend,
    T: Selectable<DB>,
{
    fn clone(&self) -> Self {
        Self {
            selection: T::construct_selection(),
            p: std::marker::PhantomData,
        }
    }
}

impl<T, DB> Copy for SelectBy<T, DB>
where
    T: Selectable<DB>,
    DB: Backend,
    T::SelectExpression: Copy,
{
}

impl<T, E, DB> QueryId for SelectBy<T, DB>
where
    DB: Backend,
    T: Selectable<DB, SelectExpression = E>,
    E: QueryId + Expression,
{
    type QueryId = E::QueryId;

    const HAS_STATIC_QUERY_ID: bool = E::HAS_STATIC_QUERY_ID;
}

impl<T, DB> SelectBy<T, DB>
where
    T: Selectable<DB>,
    DB: Backend,
{
    pub(crate) fn new() -> Self {
        Self {
            selection: T::construct_selection(),
            p: std::marker::PhantomData,
        }
    }
}

impl<T, E, DB> Expression for SelectBy<T, DB>
where
    DB: Backend,
    T: Selectable<DB, SelectExpression = E>,
    E: QueryId + Expression,
{
    type SqlType = SelectBy<T, DB>;
}

impl<T, DB> TypedExpressionType for SelectBy<T, DB>
where
    T: Selectable<DB>,
    DB: Backend,
{
}

impl<T, GB, E, DB> ValidGrouping<GB> for SelectBy<T, DB>
where
    DB: Backend,
    T: Selectable<DB, SelectExpression = E>,
    E: Expression + ValidGrouping<GB>,
{
    type IsAggregate = E::IsAggregate;
}

impl<T, DB> QueryMetadata<SelectBy<T, DB>> for DB
where
    DB: Backend,
    T: Selectable<DB>,
    DB: QueryMetadata<SqlTypeOf<T::SelectExpression>>,
{
    fn row_metadata(lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
        <DB as QueryMetadata<SqlTypeOf<<T as Selectable<DB>>::SelectExpression>>>::row_metadata(
            lookup, out,
        )
    }
}

impl<T, DB> QueryFragment<DB> for SelectBy<T, DB>
where
    T: Selectable<DB>,
    T::SelectExpression: QueryFragment<DB>,
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.selection.walk_ast(out)
    }
}

impl<T, QS, DB> SelectableExpression<QS> for SelectBy<T, DB>
where
    DB: Backend,
    T: Selectable<DB>,
    T::SelectExpression: SelectableExpression<QS>,
    Self: AppearsOnTable<QS>,
{
}

impl<T, QS, DB> AppearsOnTable<QS> for SelectBy<T, DB>
where
    DB: Backend,
    T: Selectable<DB>,
    T::SelectExpression: AppearsOnTable<QS>,
    Self: Expression,
{
}
