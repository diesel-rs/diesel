use crate::backend::DieselReserveSpecialization;
use crate::expression::*;
use crate::query_builder::*;
use crate::query_source::joins::ToInnerJoin;
use crate::result::QueryResult;
use crate::sql_types::is_nullable;
use crate::sql_types::{DieselNumericOps, IntoNullable};

#[doc(hidden)] // This is used by the `table!` macro internally
#[derive(Debug, Copy, Clone, DieselNumericOps, ValidGrouping)]
pub struct Nullable<T>(pub(crate) T);

impl<T> Nullable<T> {
    pub(crate) fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T>
where
    T: Expression,
    T::SqlType: IntoNullable,
    <T::SqlType as IntoNullable>::Nullable: TypedExpressionType,
{
    type SqlType = <T::SqlType as IntoNullable>::Nullable;
}

impl<T, DB> QueryFragment<DB> for Nullable<T>
where
    DB: Backend + DieselReserveSpecialization,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<T, QS> AppearsOnTable<QS> for Nullable<T>
where
    T: AppearsOnTable<QS>,
    Nullable<T>: Expression,
{
}

impl<T: QueryId> QueryId for Nullable<T> {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, QS> SelectableExpression<QS> for Nullable<T>
where
    Self: AppearsOnTable<QS>,
    QS: ToInnerJoin,
    T: SelectableExpression<QS::InnerJoin>,
{
}

impl<T> SelectableExpression<NoFromClause> for Nullable<T> where Self: AppearsOnTable<NoFromClause> {}

pub(crate) type NullableExpressionOf<E> =
    <<E as Expression>::SqlType as IntoNullableExpression<E>>::NullableExpression;

/// Convert expressions into their nullable form, without double wrapping them in `Nullable<...>`.
pub trait IntoNullableExpression<E> {
    /// The nullable expression of this type.
    ///
    /// For all expressions except `Nullable`, this will be `Nullable<Self>`.
    type NullableExpression;

    /// Convert this expression into its nullable representation.
    ///
    /// For `Nullable<T>`, this remain as `Nullable<T>`, otherwise the expression will be wrapped and be `Nullable<Self>`.
    fn into_nullable_expression(e: E) -> Self::NullableExpression;
}

impl<ST, E> IntoNullableExpression<E> for ST
where
    ST: SingleValue,
    E: Expression<SqlType = ST>,
    ST: SqlType,
    (ST::IsNull, ST): IntoNullableExpression<E>,
{
    type NullableExpression = <(ST::IsNull, ST) as IntoNullableExpression<E>>::NullableExpression;

    fn into_nullable_expression(e: E) -> Self::NullableExpression {
        <(ST::IsNull, ST) as IntoNullableExpression<E>>::into_nullable_expression(e)
    }
}

impl<ST, E> IntoNullableExpression<E> for (is_nullable::NotNull, ST)
where
    E: Expression<SqlType = ST>,
    ST: SqlType<IsNull = is_nullable::NotNull>,
{
    type NullableExpression = Nullable<E>;

    fn into_nullable_expression(e: E) -> Self::NullableExpression {
        Nullable(e)
    }
}

impl<ST, E> IntoNullableExpression<E> for (is_nullable::IsNullable, ST)
where
    E: Expression<SqlType = ST>,
    ST: SqlType<IsNull = is_nullable::IsNullable>,
{
    type NullableExpression = E;

    fn into_nullable_expression(e: E) -> Self::NullableExpression {
        e
    }
}
