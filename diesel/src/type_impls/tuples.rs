use crate::associations::BelongsTo;
use crate::backend::Backend;
use crate::deserialize::{
    self, FromSqlRow, FromStaticSqlRow, Queryable, SqlTypeOrSelectable, StaticallySizedRow,
};
use crate::expression::{
    is_contained_in_group_by, AppearsOnTable, AsExpression, AsExpressionList, Expression,
    IsContainedInGroupBy, QueryMetadata, Selectable, SelectableExpression, TypedExpressionType,
    ValidGrouping,
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

macro_rules! tuple_impls {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<$($T),+, __DB> HasSqlType<($($T,)+)> for __DB where
                $(__DB: HasSqlType<$T>),+,
                __DB: Backend,
            {
                fn metadata(_: &mut __DB::MetadataLookup) -> __DB::TypeMetadata {
                    unreachable!("Tuples should never implement `ToSql` directly");
                }
            }

            impl_from_sql_row!(($($T,)+), ($($ST,)+));


            impl<$($T: Expression),+> Expression for ($($T,)+)
            where ($($T::SqlType, )*): TypedExpressionType
            {
                type SqlType = ($(<$T as Expression>::SqlType,)+);
            }

            impl<$($T: TypedExpressionType,)*> TypedExpressionType for ($($T,)*) {}
            impl<$($T: SqlType + TypedExpressionType,)*> TypedExpressionType for Nullable<($($T,)*)>
            where ($($T,)*): SqlType
            {
            }

            impl<$($T: SqlType,)*> IntoNullable for ($($T,)*)
                where Self: SqlType,
            {
                type Nullable = Nullable<($($T,)*)>;
            }

            impl<$($T,)+ __DB> Selectable<__DB> for ($($T,)+)
            where
                __DB: Backend,
                $($T: Selectable<__DB>),+,
            {
                type SelectExpression = ($($T::SelectExpression,)+);

                fn construct_selection() -> Self::SelectExpression {
                    ($($T::construct_selection(),)+)
                }
            }

            impl<$($T: QueryFragment<__DB>),+, __DB: Backend> QueryFragment<__DB> for ($($T,)+) {
                #[allow(unused_assignments)]
                fn walk_ast(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
                    let mut needs_comma = false;
                    $(
                        if !self.$idx.is_noop()? {
                            if needs_comma {
                                out.push_sql(", ");
                            }
                            self.$idx.walk_ast(out.reborrow())?;
                            needs_comma = true;
                        }
                    )+
                    Ok(())
                }
            }

            impl<$($T,)+ Tab> ColumnList for ($($T,)+)
            where
                $($T: ColumnList<Table = Tab>,)+
            {
                type Table = Tab;

                fn walk_ast<__DB: Backend>(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
                    $(
                        if $idx != 0 {
                            out.push_sql(", ");
                        }
                        self.$idx.walk_ast(out.reborrow())?;
                    )+
                    Ok(())
                }
            }

            impl<$($T: QueryId),+> QueryId for ($($T,)+) {
                type QueryId = ($($T::QueryId,)+);

                const HAS_STATIC_QUERY_ID: bool = $($T::HAS_STATIC_QUERY_ID &&)+ true;
            }

            const _: () = {
                #[derive(ValidGrouping)]
                #[diesel(foreign_derive)]
                struct TupleWrapper<$($T,)*>(($($T,)*));
            };

            impl_valid_grouping_for_tuple_of_columns!($($T,)*);

            impl<$($T,)+ Tab> UndecoratedInsertRecord<Tab> for ($($T,)+)
            where
                $($T: UndecoratedInsertRecord<Tab>,)+
            {
            }

            impl<$($T,)+ __DB> CanInsertInSingleQuery<__DB> for ($($T,)+)
            where
                __DB: Backend,
                $($T: CanInsertInSingleQuery<__DB>,)+
            {
                fn rows_to_insert(&self) -> Option<usize> {
                    $(debug_assert_eq!(self.$idx.rows_to_insert(), Some(1));)+
                    Some(1)
                }
            }

            impl<$($T,)+ $($ST,)+ Tab> Insertable<Tab> for ($($T,)+)
            where
                $($T: Insertable<Tab, Values = ValuesClause<$ST, Tab>>,)+
            {
                type Values = ValuesClause<($($ST,)+), Tab>;

                fn values(self) -> Self::Values {
                    ValuesClause::new(($(self.$idx.values().values,)+))
                }
            }

            impl<'a, $($T,)+ Tab> Insertable<Tab> for &'a ($($T,)+)
            where
                ($(&'a $T,)+): Insertable<Tab>,
            {
                type Values = <($(&'a $T,)+) as Insertable<Tab>>::Values;

                fn values(self) -> Self::Values {
                    ($(&self.$idx,)+).values()
                }
            }

            #[allow(unused_assignments)]
            impl<$($T,)+ Tab, __DB> InsertValues<Tab, __DB> for ($($T,)+)
            where
                Tab: Table,
                __DB: Backend,
                $($T: InsertValues<Tab, __DB>,)+
            {
                fn column_names(&self, mut out: AstPass<__DB>) -> QueryResult<()> {
                    let mut needs_comma = false;
                    $(
                        let noop_element = self.$idx.is_noop()?;
                        if !noop_element {
                            if needs_comma {
                                out.push_sql(", ");
                            }
                            self.$idx.column_names(out.reborrow())?;
                            needs_comma = true;
                        }
                    )+
                    Ok(())
                }
            }

            impl<$($T,)+ QS> SelectableExpression<QS> for ($($T,)+) where
                $($T: SelectableExpression<QS>,)+
                ($($T,)+): AppearsOnTable<QS>,
            {
            }

            impl<$($T,)+ QS> AppearsOnTable<QS> for ($($T,)+) where
                $($T: AppearsOnTable<QS>,)+
                ($($T,)+): Expression,
            {
            }

            impl<Target, $($T,)+> AsChangeset for ($($T,)+) where
                $($T: AsChangeset<Target=Target>,)+
                Target: QuerySource,
            {
                type Target = Target;
                type Changeset = ($($T::Changeset,)+);

                fn as_changeset(self) -> Self::Changeset {
                    ($(self.$idx.as_changeset(),)+)
                }
            }

            impl<$($T,)+ Parent> BelongsTo<Parent> for ($($T,)+) where
                A: BelongsTo<Parent>,
            {
                type ForeignKey = A::ForeignKey;
                type ForeignKeyColumn = A::ForeignKeyColumn;

                fn foreign_key(&self) -> Option<&Self::ForeignKey> {
                    self.0.foreign_key()
                }

                fn foreign_key_column() -> Self::ForeignKeyColumn {
                    A::foreign_key_column()
                }
            }

            impl<$($T,)+ Next> TupleAppend<Next> for ($($T,)+) {
                type Output = ($($T,)+ Next);

                #[allow(non_snake_case)]
                fn tuple_append(self, next: Next) -> Self::Output {
                    let ($($T,)+) = self;
                    ($($T,)+ next)
                }
            }

            impl<$($T,)+ ST> AsExpressionList<ST> for ($($T,)+) where
                $($T: AsExpression<ST>,)+
                ST: SqlType + TypedExpressionType,
            {
                type Expression = ($($T::Expression,)+);

                fn as_expression_list(self) -> Self::Expression {
                    ($(self.$idx.as_expression(),)+)
                }
            }

            impl_sql_type!($($T,)*);

            impl<$($T,)* __DB, $($ST,)*> Queryable<($($ST,)*), __DB> for ($($T,)*)
            where __DB: Backend,
                  Self: FromStaticSqlRow<($($ST,)*), __DB>,
            {
                type Row = Self;

                fn build(row: Self::Row) -> deserialize::Result<Self> {
                    Ok(row)
                }
            }

            impl<__T, $($ST,)*  __DB> FromStaticSqlRow<Nullable<($($ST,)*)>, __DB> for Option<__T> where
                __DB: Backend,
                ($($ST,)*): SqlType,
                __T: FromSqlRow<($($ST,)*), __DB>,
            {

                #[allow(non_snake_case, unused_variables, unused_mut)]
                fn build_from_row<'a>(row: &impl Row<'a, __DB>)
                                      -> deserialize::Result<Self>
                {
                    match <__T as FromSqlRow<($($ST,)*), __DB>>::build_from_row(row) {
                        Ok(v) => Ok(Some(v)),
                        Err(e) if e.is::<crate::result::UnexpectedNullError>() => Ok(None),
                        Err(e) => Err(e)
                    }
                }
            }

            impl<__T,  __DB, $($ST,)*> Queryable<Nullable<($($ST,)*)>, __DB> for Option<__T>
            where __DB: Backend,
                  Self: FromStaticSqlRow<Nullable<($($ST,)*)>, __DB>,
                  ($($ST,)*): SqlType,
            {
                type Row = Self;

                fn build(row: Self::Row) -> deserialize::Result<Self> {
                    Ok(row)
                }
            }

            impl<$($T,)*> TupleSize for ($($T,)*)
                where $($T: TupleSize,)*
            {
                const SIZE: usize = $($T::SIZE +)* 0;
            }

            impl<$($T,)*> TupleSize for Nullable<($($T,)*)>
            where $($T: TupleSize,)*
                  ($($T,)*): SqlType,
            {
                const SIZE: usize = $($T::SIZE +)* 0;
            }

            impl<$($T,)* __DB> QueryMetadata<($($T,)*)> for __DB
            where __DB: Backend,
                 $(__DB: QueryMetadata<$T>,)*
            {
                fn row_metadata(lookup: &mut Self::MetadataLookup, row: &mut Vec<Option<__DB::TypeMetadata>>) {
                    $(
                        <__DB as QueryMetadata<$T>>::row_metadata(lookup, row);
                    )*
                }
            }

            impl<$($T,)* __DB> QueryMetadata<Nullable<($($T,)*)>> for __DB
            where __DB: Backend,
                  $(__DB: QueryMetadata<$T>,)*
            {
                fn row_metadata(lookup: &mut Self::MetadataLookup, row: &mut Vec<Option<__DB::TypeMetadata>>) {
                    $(
                        <__DB as QueryMetadata<$T>>::row_metadata(lookup, row);
                    )*
                }
            }

            impl<$($T,)* __DB> deserialize::QueryableByName< __DB> for ($($T,)*)
            where __DB: Backend,
            $($T: deserialize::QueryableByName<__DB>,)*
            {
                fn build<'a>(row: &impl NamedRow<'a, __DB>) -> deserialize::Result<Self> {
                    Ok(($(
                        <$T as deserialize::QueryableByName<__DB>>::build(row)?,
                    )*))
                }
            }

            impl<__T, $($ST,)* __DB> CompatibleType<__T, __DB> for ($($ST,)*)
            where
                __DB: Backend,
                __T: FromSqlRow<($($ST,)*), __DB>,
            {
                type SqlType = Self;
            }

            impl<__T, $($ST,)* __DB> CompatibleType<Option<__T>, __DB> for Nullable<($($ST,)*)>
            where
                __DB: Backend,
                ($($ST,)*): CompatibleType<__T, __DB>
            {
                type SqlType = Nullable<<($($ST,)*) as CompatibleType<__T, __DB>>::SqlType>;
            }

            impl<$($ST,)*> SqlTypeOrSelectable for ($($ST,)*)
            where $($ST: SqlTypeOrSelectable,)*
            {}

            impl<$($ST,)*> SqlTypeOrSelectable for Nullable<($($ST,)*)>
            where ($($ST,)*): SqlTypeOrSelectable
            {}

            #[cfg(feature = "postgres")]
            impl<__D, $($T,)*> crate::query_dsl::order_dsl::ValidOrderingForDistinct<crate::pg::DistinctOnClause<__D>>
                for crate::query_builder::order_clause::OrderClause<(__D, $($T,)*)> {}

        )+
    }
}

macro_rules! impl_from_sql_row {
    (($T1: ident,), ($ST1: ident,)) => {
        impl<$T1, $ST1, __DB> crate::deserialize::FromStaticSqlRow<($ST1,), __DB> for ($T1,) where
            __DB: Backend,
            $ST1: CompatibleType<$T1, __DB>,
            $T1: FromSqlRow<<$ST1 as CompatibleType<$T1, __DB>>::SqlType, __DB>,
        {

            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn build_from_row<'a>(row: &impl Row<'a, __DB>)
                                                       -> deserialize::Result<Self>
            {
                Ok(($T1::build_from_row(row)?,))
            }
        }
    };
    (($T1: ident, $($T: ident,)*), ($ST1: ident, $($ST: ident,)*)) => {
        impl<$T1, $($T,)* $($ST,)* __DB> FromSqlRow<($($ST,)* crate::sql_types::Untyped), __DB> for ($($T,)* $T1)
        where __DB: Backend,
              $T1: FromSqlRow<crate::sql_types::Untyped, __DB>,
            $(
                $T: FromSqlRow<$ST, __DB> + StaticallySizedRow<$ST, __DB>,
        )*
        {
            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn build_from_row<'a>(full_row: &impl Row<'a, __DB>)
                -> deserialize::Result<Self>
            {
                let field_count = full_row.field_count();

                let mut static_field_count = 0;
                $(
                    let row = full_row.partial_row(static_field_count..static_field_count + $T::FIELD_COUNT);
                    static_field_count += $T::FIELD_COUNT;
                    let $T = $T::build_from_row(&row)?;
                )*

                let row = full_row.partial_row(static_field_count..field_count);

                Ok(($($T,)* $T1::build_from_row(&row)?,))
            }
        }

        impl<$T1, $ST1, $($T,)* $($ST,)* __DB> FromStaticSqlRow<($($ST,)* $ST1,), __DB> for ($($T,)* $T1,) where
            __DB: Backend,
            $ST1: CompatibleType<$T1, __DB>,
            $T1: FromSqlRow<<$ST1 as CompatibleType<$T1, __DB>>::SqlType, __DB>,
            $(
                $ST: CompatibleType<$T, __DB>,
                $T: FromSqlRow<<$ST as CompatibleType<$T, __DB>>::SqlType, __DB> + StaticallySizedRow<<$ST as CompatibleType<$T, __DB>>::SqlType, __DB>,
            )*

        {

            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn build_from_row<'a>(full_row: &impl Row<'a, __DB>)
                -> deserialize::Result<Self>
            {
                let field_count = full_row.field_count();

                let mut static_field_count = 0;
                $(
                    let row = full_row.partial_row(static_field_count..static_field_count + $T::FIELD_COUNT);
                    static_field_count += $T::FIELD_COUNT;
                    let $T = <$T as FromSqlRow<<$ST as CompatibleType<$T, __DB>>::SqlType, __DB>>::build_from_row(&row)?;
                )*

                let row = full_row.partial_row(static_field_count..field_count);

                Ok(($($T,)* $T1::build_from_row(&row)?,))
            }
        }
    }
}

macro_rules! impl_valid_grouping_for_tuple_of_columns {
    (
        @build
        start_ts = [$($ST: ident,)*],
        ts = [$T1: ident,],
        bounds = [$($bounds: tt)*],
        is_aggregate = [$($is_aggregate: tt)*],
    ) => {
        impl<$($ST,)* Col> IsContainedInGroupBy<Col> for ($($ST,)*)
        where Col: Column,
            $($ST: IsContainedInGroupBy<Col>,)*
            $($bounds)*
            <$T1 as IsContainedInGroupBy<Col>>::Output: is_contained_in_group_by::IsAny<$($is_aggregate)*>,
        {
            type Output = <<$T1 as IsContainedInGroupBy<Col>>::Output as is_contained_in_group_by::IsAny<$($is_aggregate)*>>::Output;
        }
    };
    (
        @build
        start_ts = [$($ST: ident,)*],
        ts = [$T1: ident, $($T: ident,)+],
        bounds = [$($bounds: tt)*],
        is_aggregate = [$($is_aggregate: tt)*],
    ) => {
        impl_valid_grouping_for_tuple_of_columns! {
            @build
            start_ts = [$($ST,)*],
            ts = [$($T,)*],
            bounds = [
                $($bounds)*
                <$T1 as IsContainedInGroupBy<Col>>::Output: is_contained_in_group_by::IsAny<$($is_aggregate)*>,
            ],
            is_aggregate = [
                <<$T1 as IsContainedInGroupBy<Col>>::Output as is_contained_in_group_by::IsAny<$($is_aggregate)*>>::Output
            ],
        }
    };
    ($T1: ident, $($T: ident,)+) => {
        impl_valid_grouping_for_tuple_of_columns! {
            @build
            start_ts = [$T1, $($T,)*],
            ts = [$($T,)*],
            bounds = [],
            is_aggregate = [<$T1 as IsContainedInGroupBy<Col>>::Output],
        }
    };
    ($T1: ident,) => {
        impl<$T1, Col> IsContainedInGroupBy<Col> for ($T1,)
        where Col: Column,
              $T1: IsContainedInGroupBy<Col>
        {
            type Output = <$T1 as IsContainedInGroupBy<Col>>::Output;
        }
    };
}

macro_rules! impl_sql_type {
    (
        @build
        start_ts = [$($ST: ident,)*],
        ts = [$T1: ident,],
        bounds = [$($bounds: tt)*],
        is_null = [$($is_null: tt)*],
    )=> {
        impl<$($ST,)*> SqlType for ($($ST,)*)
        where
            $($ST: SqlType,)*
            $($bounds)*
            $T1::IsNull: OneIsNullable<$($is_null)*>,
        {
            type IsNull = <$T1::IsNull as OneIsNullable<$($is_null)*>>::Out;
        }

    };
    (
        @build
        start_ts = [$($ST: ident,)*],
        ts = [$T1: ident, $($T: ident,)+],
        bounds = [$($bounds: tt)*],
        is_null = [$($is_null: tt)*],
    )=> {
        impl_sql_type!{
            @build
            start_ts = [$($ST,)*],
            ts = [$($T,)*],
            bounds = [$($bounds)* $T1::IsNull: OneIsNullable<$($is_null)*>,],
            is_null = [<$T1::IsNull as OneIsNullable<$($is_null)*>>::Out],
        }
    };
    ($T1: ident, $($T: ident,)+) => {
        impl_sql_type!{
            @build
            start_ts = [$T1, $($T,)*],
            ts = [$($T,)*],
            bounds = [],
            is_null = [$T1::IsNull],
        }
    };
    ($T1: ident,) => {
        impl<$T1> SqlType for ($T1,)
        where $T1: SqlType,
        {
            type IsNull = $T1::IsNull;
        }
    }
}

__diesel_for_each_tuple!(tuple_impls);
