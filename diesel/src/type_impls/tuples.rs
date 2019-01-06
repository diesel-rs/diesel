use std::error::Error;

use crate::associations::BelongsTo;
use crate::backend::Backend;
use crate::deserialize::{self, FromSqlRow, Queryable, QueryableByName};
use crate::expression::{
    AppearsOnTable, AsExpression, AsExpressionList, Expression, NonAggregate, SelectableExpression,
};
use crate::insertable::{CanInsertInSingleQuery, InsertValues, Insertable};
use crate::query_builder::*;
use crate::query_source::*;
use crate::result::QueryResult;
use crate::row::*;
use crate::sql_types::{HasSqlType, NotNull};
use crate::util::TupleAppend;

#[cfg(feature = "mysql")]
use crate::sql_types::IsSigned;

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
                fn metadata(_: &__DB::MetadataLookup) -> __DB::TypeMetadata {
                    unreachable!("Tuples should never implement `ToSql` directly");
                }

                #[cfg(feature = "with-deprecated")]
                #[allow(deprecated)]
                fn row_metadata(out: &mut Vec<__DB::TypeMetadata>, lookup: &__DB::MetadataLookup) {
                    $(<__DB as HasSqlType<$T>>::row_metadata(out, lookup);)+
                }

                #[cfg(feature = "mysql")]
                fn mysql_row_metadata(out: &mut Vec<(__DB::TypeMetadata, IsSigned)>, lookup: &__DB::MetadataLookup) {
                    $(<__DB as HasSqlType<$T>>::mysql_row_metadata(out, lookup);)+
                }
            }

            impl<$($T),+> NotNull for ($($T,)+) {
            }

            impl<$($T),+, $($ST),+, __DB> FromSqlRow<($($ST,)+), __DB> for ($($T,)+) where
                __DB: Backend,
                $($T: FromSqlRow<$ST, __DB>),+,
            {
                const FIELDS_NEEDED: usize = $($T::FIELDS_NEEDED +)+ 0;

                fn build_from_row<RowT: Row<__DB>>(row: &mut RowT) -> Result<Self, Box<Error+Send+Sync>> {
                    Ok(($($T::build_from_row(row)?,)+))
                }
            }

            impl<$($T),+, $($ST),+, __DB> Queryable<($($ST,)+), __DB> for ($($T,)+) where
                __DB: Backend,
                $($T: Queryable<$ST, __DB>),+,
            {
                type Row = ($($T::Row,)+);

                fn build(row: Self::Row) -> Self {
                    ($($T::build(row.$idx),)+)
                }
            }

            impl<$($T,)+ __DB> QueryableByName<__DB> for ($($T,)+)
            where
                __DB: Backend,
                $($T: QueryableByName<__DB>,)+
            {
                fn build<RowT: NamedRow<__DB>>(row: &RowT) -> deserialize::Result<Self> {
                    Ok(($($T::build(row)?,)+))
                }
            }

            impl<$($T: Expression + NonAggregate),+> Expression for ($($T,)+) {
                type SqlType = ($(<$T as Expression>::SqlType,)+);
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

            impl<$($T: Expression + NonAggregate),+> NonAggregate for ($($T,)+) {
            }

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
            {
                type Expression = ($($T::Expression,)+);

                fn as_expression_list(self) -> Self::Expression {
                    ($(self.$idx.as_expression(),)+)
                }
            }
        )+
    }
}

__diesel_for_each_tuple!(tuple_impls);
