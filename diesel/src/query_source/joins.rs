use query_builder::*;
use result::QueryResult;
use super::{QuerySource, Table, Column};
use types::{IntoNullable, NotNull};
use query_builder::nodes::Join;
use expression::nullable::Nullable;
use expression::expression_methods::global_expression_methods::ExpressionMethods;
use expression::{Expression, SelectableExpression};

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct InnerJoinSource<Left, Right, FK> {
    left: Left,
    right: Right,
    fk: ::std::marker::PhantomData<FK>,
}

// TODO: make this not conflicting to allow more than one join between two tables!!
//       maybe it's possible to solve this with specialization

// impl<Left, Other, Right, FK, OtherFK, JoinType> JoinTo<Other, JoinType, OtherFK> for InnerJoinSource<Left, Right, FK>
//     where OtherFK: Column<SqlType = <<<Left as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType>,
//           FK: Column<SqlType = <<<Left as JoinTo<Right, Inner, FK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType>,
//           Right: Table,
//           Other: Table,
//           Left: JoinTo<Right, Inner, FK> + JoinTo<Other, JoinType, OtherFK>,
//           <<<Left as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType: NotNull,
//          <<<Left as JoinTo<Right, Inner, FK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType: NotNull,
// {

//     type JoinSqlType = (<Left as JoinTo<Right, Inner, FK>>::JoinSqlType,
//                         <Other as AsQuery>::SqlType);
//     type JoinAllColumns = (<Left as JoinTo<Right, Inner, FK>>::JoinAllColumns,
//                            <Other as Table>::AllColumns);

// type ParentTable = <Left as JoinTo<Other, JoinType, OtherFK>>::ParentTable;

//     type JoinClause = Join<<Left as JoinTo<Right, Inner, FK>>::JoinClause,
//                            <Other as QuerySource>::FromClause,
//                            ::expression::helper_types::Eq<Nullable<OtherFK>,
//                                     Nullable<<<Left as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey>>,
//                            JoinType>;

//     #[doc(hidden)]
//     fn join_clause(&self, join_type: JoinType) -> Self::JoinClause {
//         let fk = OtherFK::default();
//         let parent_table = Self::ParentTable::default();
//         let other = Other::default();
//         Join::new(<Left as JoinTo<Right, Inner, FK>>::join_clause(&self.left, Inner),
//                   other.from_clause(),
//                   ExpressionMethods::eq(fk.nullable(), parent_table.primary_key().nullable()),
//                   join_type)
//     }

//     fn join_all_columns() -> Self::JoinAllColumns{
//         (<Left as JoinTo<Right, Inner, FK>>::join_all_columns(), <Other as Table>::all_columns())
//     }
// }

impl<Left, Other, Right, FK, OtherFK, JoinType> JoinTo<Other, JoinType, OtherFK> for InnerJoinSource<Left, Right, FK> where
    OtherFK: Column<SqlType =
        <<<Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType>,
    FK: Column<SqlType =
        <<<Left as JoinTo<Right, Inner, FK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType>,
    Right: Table + JoinTo<Other, JoinType, OtherFK>,
    Other: Table,
    Left: JoinTo<Right, Inner, FK>,
    <<<Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType: NotNull,
    <<<Left as JoinTo<Right, Inner, FK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType: NotNull,
{

    type JoinSqlType = (<Left as JoinTo<Right, Inner, FK>>::JoinSqlType,
        <Other as AsQuery>::SqlType);
    type JoinAllColumns = (<Left as JoinTo<Right, Inner, FK>>::JoinAllColumns,
        <Other as Table>::AllColumns);

    type ParentTable = <Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable;

    type JoinClause = Join<<Left as JoinTo<Right, Inner, FK>>::JoinClause,
        <Other as QuerySource>::FromClause,
        ::expression::helper_types::Eq<Nullable<OtherFK>,
            Nullable<<<Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey>>,
        JoinType>;

    #[doc(hidden)]
    fn join_clause(&self, join_type: JoinType) -> Self::JoinClause {
        let fk = OtherFK::default();
        let parent_table = Self::ParentTable::default();
        let other = Other::default();
        Join::new(<Left as JoinTo<Right, Inner, FK>>::join_clause(&self.left, Inner),
            other.from_clause(),
            ExpressionMethods::eq(fk.nullable(), parent_table.primary_key().nullable()),
            join_type)
    }

    fn join_all_columns() -> Self::JoinAllColumns{
        (<Left as JoinTo<Right, Inner, FK>>::join_all_columns(),
            <Other as Table>::all_columns())
    }
}

impl<Left, Right, FK> InnerJoinSource<Left, Right, FK> where
    Left: JoinTo<Right, Inner, FK>,
    Right: Table,
    FK: Column
{
    pub fn new(left: Left, right: Right, _: FK) -> Self {
        InnerJoinSource {
            left: left,
            right: right,
            fk: ::std::marker::PhantomData,
        }
    }
}


impl<Left, Right, FK> QuerySource for InnerJoinSource<Left, Right, FK> where
    Left: QuerySource + JoinTo<Right, Inner, FK>,
    Right: Table,
    FK: Column
{
    type FromClause = <Left as JoinTo<Right, Inner, FK>>::JoinClause;

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(Inner)
    }
}

impl<Left, Right, FK> AsQuery for InnerJoinSource<Left, Right, FK> where
    Left: JoinTo<Right, Inner, FK>,
    Right: Table,
    FK: Column,
    <Left as JoinTo<Right, Inner, FK>>::JoinAllColumns:
        SelectableExpression<InnerJoinSource<Left, Right, FK>,
            <Left as JoinTo<Right, Inner, FK>>::JoinSqlType>
{
    type SqlType = <Left as JoinTo<Right, Inner, FK>>::JoinSqlType;
    type Query = SelectStatement<<Left as JoinTo<Right, Inner, FK>>::JoinSqlType,
        <Left as JoinTo<Right, Inner, FK>>::JoinAllColumns, Self>;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(Left::join_all_columns(), self)
    }
}


impl_query_id!(InnerJoinSource<Left, Right, FK>);

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct LeftOuterJoinSource<Left, Right, FK> {
    left: Left,
    right: Right,
    fk: ::std::marker::PhantomData<FK>,
}


impl<Left, Right, FK> LeftOuterJoinSource<Left, Right, FK> where
    Left: JoinTo<Right, LeftOuter, FK>,
    Right: Table,
    Right::SqlType: IntoNullable,
    FK: Column,
    <Left as JoinTo<Right, LeftOuter, FK>>::JoinAllColumns:
        SelectableExpression<LeftOuterJoinSource<Left, Right, FK>,
            <Left as JoinTo<Right, LeftOuter, FK>>::JoinSqlType,>,
{
    pub fn new(left: Left, right: Right, _: FK) -> Self {
        LeftOuterJoinSource {
            left: left,
            right: right,
            fk: ::std::marker::PhantomData,
        }
    }
}

impl<Left, Right, FK> QuerySource for LeftOuterJoinSource<Left, Right, FK> where
    Left: QuerySource + JoinTo<Right, LeftOuter, FK>,
    Right: Table,
    FK: Column
{
    type FromClause = <Left as JoinTo<Right, LeftOuter, FK>>::JoinClause;

    fn from_clause(&self) -> Self::FromClause {
        self.left.join_clause(LeftOuter)
    }
}

impl<Left, Right, FK> AsQuery for LeftOuterJoinSource<Left, Right, FK> where
    Left: JoinTo<Right, LeftOuter, FK>,
    Right: Table,
    Right::SqlType: IntoNullable,
    FK: Column,
    <Left as JoinTo<Right, LeftOuter, FK>>::JoinAllColumns:
        SelectableExpression<LeftOuterJoinSource<Left, Right, FK>,
            <Left as JoinTo<Right, LeftOuter, FK>>::JoinSqlType,>,
{
    type SqlType = <Left as JoinTo<Right, LeftOuter, FK>>::JoinSqlType;
    type Query = SelectStatement<
        <Left as JoinTo<Right, LeftOuter, FK>>::JoinSqlType,
        <Left as JoinTo<Right, LeftOuter, FK>>::JoinAllColumns,
        Self,
    >;

    fn as_query(self) -> Self::Query {
        SelectStatement::simple(Left::join_all_columns(), self)
    }
}

impl<Left, Other, Right, FK, OtherFK, JoinType> JoinTo<Other, JoinType, OtherFK> for LeftOuterJoinSource<Left, Right, FK> where
    OtherFK: Column<SqlType =
        <<<Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType>,
    FK: Column<SqlType =
        <<<Left as JoinTo<Right, LeftOuter, FK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType>,
    Right: Table + JoinTo<Other, JoinType, OtherFK>,
    Right::SqlType: IntoNullable,
    Other: Table,
    Left: JoinTo<Right, LeftOuter, FK>,
    <<<Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType: NotNull,
    <<<Left as JoinTo<Right, LeftOuter, FK>>::ParentTable as Table>::PrimaryKey as Expression>::SqlType: NotNull,
    <Other as AsQuery>::SqlType: NotNull,
{

    type JoinSqlType = (<Left as JoinTo<Right, LeftOuter, FK>>::JoinSqlType,
        <<Other as AsQuery>::SqlType as IntoNullable>::Nullable);
    type JoinAllColumns = (<Left as JoinTo<Right, LeftOuter, FK>>::JoinAllColumns,
        <Other as Table>::AllColumns);

    type ParentTable = <Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable;

    type JoinClause = Join<<Left as JoinTo<Right, LeftOuter, FK>>::JoinClause,
        <Other as QuerySource>::FromClause,
        ::expression::helper_types::Eq<Nullable<OtherFK>,
            Nullable<<<Right as JoinTo<Other, JoinType, OtherFK>>::ParentTable as Table>::PrimaryKey>>,
        JoinType>;

    #[doc(hidden)]
    fn join_clause(&self, join_type: JoinType) -> Self::JoinClause {
        let fk = OtherFK::default();
        let parent_table = Self::ParentTable::default();
        let other = Other::default();
        Join::new(<Left as JoinTo<Right, LeftOuter, FK>>::join_clause(&self.left, LeftOuter),
            other.from_clause(),
            ExpressionMethods::eq(fk.nullable(), parent_table.primary_key().nullable()),
            join_type)
    }

    fn join_all_columns() -> Self::JoinAllColumns{
        (<Left as JoinTo<Right, LeftOuter, FK>>::join_all_columns(),
            <Other as Table>::all_columns())
    }
}

impl_query_id!(LeftOuterJoinSource<Left, Right, FK>);

/// Indicates that two tables can be used together in a JOIN clause.
/// Implementations of this trait will be generated for you automatically by
/// the [association annotations](FIXME: Add link) from codegen.
pub trait JoinTo<T: Table, JoinType, FK: Column> {
    type ParentTable: Table;
    type JoinSqlType;
    type JoinAllColumns;

    #[doc(hidden)]
    type JoinClause;
    #[doc(hidden)]
    fn join_clause(&self, join_type: JoinType) -> Self::JoinClause;

    fn join_all_columns() -> Self::JoinAllColumns;
}




pub trait InnerJoinable: Sized {
    fn inner_join<T, FK>(self, other: T, by: FK) -> InnerJoinSource<Self, T, FK> where
        T: Table,
        FK: Column,
        Self: JoinTo<T, Inner, FK>,
        Self::JoinAllColumns: SelectableExpression<
            InnerJoinSource<Self, T, FK>,
            <Self as JoinTo<T, Inner, FK>>::JoinSqlType>
    {
        InnerJoinSource::new(self, other, by)
    }
}

pub trait LeftJoinable: Sized {
    fn left_outer_join<T, FK>(self, other: T, by: FK) -> LeftOuterJoinSource<Self, T, FK> where
        Self: JoinTo<T, LeftOuter, FK>,
        T: Table,
        FK: Column,
        T::SqlType: IntoNullable,
        Self::JoinAllColumns: SelectableExpression<
            LeftOuterJoinSource<Self, T, FK>,
            <Self as JoinTo<T, LeftOuter, FK>>::JoinSqlType>
    {
        LeftOuterJoinSource::new(self, other, by)
    }
}

impl<T> InnerJoinable for T where T: Table {}
impl<Left, Right, FK> InnerJoinable for InnerJoinSource<Left, Right, FK> {}
impl<Left, Right, FK> InnerJoinable for LeftOuterJoinSource<Left, Right, FK> {}
impl<T> LeftJoinable for T where T: Table {}
impl<Left, Right, FK> LeftJoinable for InnerJoinSource<Left, Right, FK> {}
impl<Left, Right, FK> LeftJoinable for LeftOuterJoinSource<Left, Right, FK> {}

use backend::Backend;

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct Inner;

impl<DB: Backend> QueryFragment<DB> for Inner {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(" INNER");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct LeftOuter;


impl<DB: Backend> QueryFragment<DB> for LeftOuter {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(" LEFT OUTER");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}
