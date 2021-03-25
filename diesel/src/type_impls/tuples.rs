#![allow(non_camel_case_types, dead_code)]
use crate::associations::BelongsTo;
use crate::backend::Backend;
use crate::deserialize::{
    self, FromSqlRow, FromStaticSqlRow, Queryable, SqlTypeOrSelectable, StaticallySizedRow,
};
use crate::expression::{
    is_contained_in_group_by, AppearsOnTable, AsExpression, AsExpressionList, Expression,
    IsContainedInGroupBy, QueryMetadata, SelectableExpression, TypedExpressionType, ValidGrouping,
};
use crate::insertable::{CanInsertInSingleQuery, InsertValues, Insertable};
use crate::query_builder::*;
use crate::query_dsl::load_dsl::CompatibleType;
use crate::query_source::*;
use crate::result::QueryResult;
use crate::row::*;
use crate::sql_types::{HasSqlType, IntoNullable, Nullable, OneIsNullable, SqlType};
use crate::util::{TupleAppend, TupleSize};

impl<T> TupleSize for T
where
    T: crate::sql_types::SingleValue,
{
    const SIZE: usize = 1;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, __DB> HasSqlType<(T,)> for __DB
where
    __DB: HasSqlType<T>,
    __DB: Backend,
{
    fn metadata(_: &__DB::MetadataLookup) -> __DB::TypeMetadata {
        unreachable!("Tuples should never implement `ToSql` directly");
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: Expression> Expression for (T,)
where
    (T::SqlType,): TypedExpressionType,
{
    type SqlType = (<T as Expression>::SqlType,);
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: TypedExpressionType> TypedExpressionType for (T,) {}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: SqlType + TypedExpressionType> TypedExpressionType for Nullable<(T,)> where
    (T,): SqlType
{
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: SqlType> IntoNullable for (T,)
where
    Self: SqlType,
{
    type Nullable = Nullable<(T,)>;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: UndecoratedInsertRecord<Tab>, Tab> UndecoratedInsertRecord<Tab> for (T,) {}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: SelectableExpression<QS>, QS> SelectableExpression<QS> for (T,) where
    (T,): AppearsOnTable<QS>
{
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: AppearsOnTable<QS>, QS> AppearsOnTable<QS> for (T,) where (T,): Expression {}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, #[repeat] ST, __DB> Queryable<(ST,), __DB> for (T,)
where
    __DB: Backend,
    Self: FromStaticSqlRow<(ST,), __DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: QueryId> QueryId for (T,) {
    type QueryId = (T::QueryId,);

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID && true;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: QueryFragment<__DB>, __DB: Backend> QueryFragment<__DB> for (T,) {
    #[allow(unused_assignments)]
    fn walk_ast(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
        let mut needs_comma = false;

        #[repeat]
        {
            if !self.idx.is_noop()? {
                if needs_comma {
                    out.push_sql(", ");
                }
                self.idx.walk_ast(out.reborrow())?;
                needs_comma = true;
            }
        }
        Ok(())
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<__T, #[repeat] ST, __DB> FromStaticSqlRow<Nullable<(ST,)>, __DB> for Option<__T>
where
    __DB: Backend,
    (ST,): SqlType,
    __T: FromSqlRow<(ST,), __DB>,
{
    #[allow(non_snake_case, unused_variables, unused_mut)]
    fn build_from_row<'a>(row: &impl Row<'a, __DB>) -> deserialize::Result<Self> {
        match <__T as FromSqlRow<(ST,), __DB>>::build_from_row(row) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.is::<crate::result::UnexpectedNullError>() => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, Tab> ColumnList for (T,)
where
    T: ColumnList<Table = Tab>,
{
    type Table = Tab;

    #[allow(unused_assignments)]
    fn walk_ast<__DB: Backend>(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
        let mut needs_comma = false;
        #[repeat]
        {
            if needs_comma {
                out.push_sql(", ");
            }
            needs_comma = true;
            self.idx.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: CanInsertInSingleQuery<__DB>, __DB> CanInsertInSingleQuery<__DB> for (T,)
where
    __DB: Backend,
{
    fn rows_to_insert(&self) -> Option<usize> {
        #[repeat]
        {
            let val = self.idx.rows_to_insert();
            debug_assert_eq!(val, Some(1));
        }
        Some(1)
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, #[repeat] ST, Tab> Insertable<Tab> for (T,)
where
    T: Insertable<Tab, Values = ValuesClause<ST, Tab>>,
{
    type Values = ValuesClause<(ST,), Tab>;

    fn values(self) -> Self::Values {
        ValuesClause::new(
            #[repeat]
            (self.idx.values().values,),
        )
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<'a, #[repeat] T, Tab> Insertable<Tab> for &'a (T,)
where
    (&'a T,): Insertable<Tab>,
{
    type Values = <(&'a T,) as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        let values = #[repeat]
        (&self.idx,);
        values.values()
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, Tab, __DB> InsertValues<Tab, __DB> for (T,)
where
    Tab: Table,
    __DB: Backend,
    T: InsertValues<Tab, __DB>,
{
    #[allow(unused_assignments)]
    fn column_names(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
        let mut needs_comma = false;
        #[repeat]
        {
            let noop_element = self.idx.is_noop()?;
            if !noop_element {
                if needs_comma {
                    out.push_sql(", ");
                }
                self.idx.column_names(out.reborrow())?;
                needs_comma = true;
            }
        }
        Ok(())
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<Target, #[repeat] T> AsChangeset for (T,)
where
    T: AsChangeset<Target = Target>,
    Target: QuerySource,
{
    type Target = Target;
    type Changeset = (T::Changeset,);

    fn as_changeset(self) -> Self::Changeset {
        #[repeat]
        (self.idx.as_changeset(),)
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, Parent> BelongsTo<Parent> for (T,)
where
    T_0: BelongsTo<Parent>,
{
    type ForeignKey = T_0::ForeignKey;
    type ForeignKeyColumn = T_0::ForeignKeyColumn;

    fn foreign_key(&self) -> Option<&Self::ForeignKey> {
        self.0.foreign_key()
    }

    fn foreign_key_column() -> Self::ForeignKeyColumn {
        T_0::foreign_key_column()
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, ST> AsExpressionList<ST> for (T,)
where
    T: AsExpression<ST>,
    ST: SqlType + TypedExpressionType,
{
    type Expression = (T::Expression,);

    fn as_expression_list(self) -> Self::Expression {
        #[repeat]
        (self.idx.as_expression(),)
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<__T, __DB, #[repeat] ST> Queryable<Nullable<(ST,)>, __DB> for Option<__T>
where
    __DB: Backend,
    Self: FromStaticSqlRow<Nullable<(ST,)>, __DB>,
    (ST,): SqlType,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, __DB> QueryMetadata<(T,)> for __DB
where
    __DB: Backend,
    __DB: QueryMetadata<T>,
{
    fn row_metadata(lookup: &Self::MetadataLookup, row: &mut Vec<Option<__DB::TypeMetadata>>) {
        #[repeat]
        {
            <__DB as QueryMetadata<T>>::row_metadata(lookup, row);
        }
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, __DB> QueryMetadata<Nullable<(T,)>> for __DB
where
    __DB: Backend,
    __DB: QueryMetadata<T>,
{
    fn row_metadata(lookup: &Self::MetadataLookup, row: &mut Vec<Option<__DB::TypeMetadata>>) {
        #[repeat]
        {
            <__DB as QueryMetadata<T>>::row_metadata(lookup, row);
        }
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, __DB> deserialize::QueryableByName<__DB> for (T,)
where
    __DB: Backend,
    T: deserialize::QueryableByName<__DB>,
{
    fn build<'a>(row: &impl NamedRow<'a, __DB>) -> deserialize::Result<Self> {
        Ok(
            #[repeat]
            (<T as deserialize::QueryableByName<__DB>>::build(row)?,),
        )
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: TupleSize> TupleSize for (T,) {
    const SIZE: usize = T::SIZE + 0;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T: TupleSize> TupleSize for Nullable<(T,)>
where
    Self: SqlType,
{
    const SIZE: usize = T::SIZE + 0;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] T, Next> TupleAppend<Next> for (T,) {
    type Output = (T, Next);

    #[allow(non_snake_case)]
    fn tuple_append(self, next: Next) -> Self::Output {
        #[repeat]
        (self.idx, next)
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<T, #[repeat] ST, DB> CompatibleType<T, DB> for (ST,)
where
    DB: Backend,
    T: FromSqlRow<(ST,), DB>,
{
    type SqlType = Self;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<T, #[repeat] ST, DB> CompatibleType<Option<T>, DB> for Nullable<(ST,)>
where
    DB: Backend,
    (ST,): CompatibleType<T, DB>,
{
    type SqlType = Nullable<<(ST,) as CompatibleType<T, DB>>::SqlType>;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] ST> SqlTypeOrSelectable for (ST,) where ST: SqlTypeOrSelectable {}

#[diesel_derives::__diesel_for_each_tuple]
impl<#[repeat] ST> SqlTypeOrSelectable for Nullable<(ST,)> where (ST,): SqlTypeOrSelectable {}

#[cfg(feature = "postgres")]
#[diesel_derives::__diesel_for_each_tuple]
impl<__D, #[repeat] T>
    crate::query_dsl::order_dsl::ValidOrderingForDistinct<crate::pg::DistinctOnClause<__D>>
    for crate::query_builder::order_clause::OrderClause<(__D, T)>
{
}

#[diesel_derives::__diesel_for_each_tuple]
#[derive(ValidGrouping)]
#[diesel(foreign_derive)]
struct TupleWrapper<#[repeat] T>((T,));

impl<T1, ST1, __DB> crate::deserialize::FromStaticSqlRow<(ST1,), __DB> for (T1,)
where
    __DB: Backend,
    ST1: CompatibleType<T1, __DB>,
    T1: FromSqlRow<<ST1 as CompatibleType<T1, __DB>>::SqlType, __DB>,
{
    fn build_from_row<'a>(row: &impl Row<'a, __DB>) -> deserialize::Result<Self> {
        Ok((T1::build_from_row(row)?,))
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<T1, #[repeat] T, #[repeat] ST, DB> FromSqlRow<(ST, crate::sql_types::Untyped), DB> for (T, T1)
where
    DB: Backend,
    T1: FromSqlRow<crate::sql_types::Untyped, DB>,
    T: FromSqlRow<ST, DB> + StaticallySizedRow<ST, DB>,
{
    #[allow(non_snake_case, unused_variables, unused_mut)]
    fn build_from_row<'a>(full_row: &impl Row<'a, DB>) -> deserialize::Result<Self> {
        let field_count = full_row.field_count();

        let mut static_field_count = 0;

        Ok(
            #[repeat]
            (
                {
                    let row = full_row
                        .partial_row(static_field_count..static_field_count + T::FIELD_COUNT);
                    static_field_count += T::FIELD_COUNT;
                    T::build_from_row(&row)?
                },
                {
                    let row = full_row.partial_row(static_field_count..field_count);
                    T1::build_from_row(&row)?
                },
            ),
        )
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<T1, ST1, #[repeat] T, #[repeat] ST, DB> FromStaticSqlRow<(ST, ST1), DB> for (T, T1)
where
    DB: Backend,
    ST1: CompatibleType<T1, DB>,
    T1: FromSqlRow<<ST1 as CompatibleType<T1, DB>>::SqlType, DB>,
    ST: CompatibleType<T, DB>,
    T: FromSqlRow<<ST as CompatibleType<T, DB>>::SqlType, DB>
        + StaticallySizedRow<<ST as CompatibleType<T, DB>>::SqlType, DB>,
{
    #[allow(non_snake_case, unused_variables, unused_mut)]
    fn build_from_row<'a>(full_row: &impl Row<'a, DB>) -> deserialize::Result<Self> {
        let field_count = full_row.field_count();
        let mut static_field_count = 0;
        Ok(
            #[repeat]
            (
                {
                    let row = full_row
                        .partial_row(static_field_count..static_field_count + T::FIELD_COUNT);
                    static_field_count += T::FIELD_COUNT;
                    T::build_from_row(&row)?
                },
                {
                    let row = full_row.partial_row(static_field_count..field_count);
                    T1::build_from_row(&row)?
                },
            ),
        )
    }
}

impl<ST1> SqlType for (ST1,)
where
    ST1: SqlType,
{
    type IsNull = ST1::IsNull;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<ST1, #[repeat] ST> SqlType for (ST, ST1)
where
    ST1: SqlType,
    ST: SqlType,
    ST1::IsNull: OneIsNullable<<(ST,) as SqlType>::IsNull>,
{
    type IsNull = <ST1::IsNull as OneIsNullable<<(ST,) as SqlType>::IsNull>>::Out;
}

impl<T1, Col> IsContainedInGroupBy<Col> for (T1,)
where
    Col: Column,
    T1: IsContainedInGroupBy<Col>,
{
    type Output = <T1 as IsContainedInGroupBy<Col>>::Output;
}

#[diesel_derives::__diesel_for_each_tuple]
impl<T1, #[repeat] T, Col> IsContainedInGroupBy<Col> for (T, T1)
where
    Col: Column,
    T1: IsContainedInGroupBy<Col>,
    (T,): IsContainedInGroupBy<Col>,
    <T1 as IsContainedInGroupBy<Col>>::Output:
        is_contained_in_group_by::IsAny<<(T,) as IsContainedInGroupBy<Col>>::Output>,
{
    type Output = <<T1 as IsContainedInGroupBy<Col>>::Output as is_contained_in_group_by::IsAny<
        <(T,) as IsContainedInGroupBy<Col>>::Output,
    >>::Output;
}
