// use super::BoxedSelectByStatement;
use crate::associations::HasTable;
use crate::deserialize::TableQueryable;
// use crate::backend::Backend;
// use crate::expression::nullable::Nullable;
use crate::expression::*;
use crate::insertable::Insertable;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::{Query, SelectByQuery, SelectByStatement};
// use crate::query_dsl::boxed_dsl::BoxedDsl;
use crate::query_dsl::methods::*;
use crate::query_dsl::*;
// use crate::query_source::joins::{Join, JoinOn, JoinTo};
use crate::query_source::joins::JoinTo;
// use crate::query_source::QuerySource;
use crate::sql_types::Bool;

impl<S, Stmt, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for SelectByStatement<S, Stmt>
where
    Stmt: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = Stmt::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.inner.join(rhs, kind, on)
    }
}

impl<S, Stmt, Selection> SelectDsl<Selection> for SelectByStatement<S, Stmt>
where
    Selection: Expression,
    Stmt: SelectDsl<Selection>,
{
    type Output = Stmt::Output;

    fn select(self, selection: Selection) -> Self::Output {
        self.inner.select(selection)
    }
}

impl<S, Stmt, Selection> SelectByDsl<Selection> for SelectByStatement<S, Stmt>
where
    Selection: TableQueryable,
    Stmt: SelectByDsl<Selection>,
{
    type Output = Stmt::Output;

    fn select_by(self) -> Self::Output {
        self.inner.select_by()
    }
}

impl<CL, S, Stmt> DistinctDsl for SelectByStatement<S, Stmt>
where
    Self: SelectByQuery<Columns = CL>,
    Stmt: DistinctDsl,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn distinct(self) -> Self::Output {
        SelectByStatement::new(self.inner.distinct())
    }
}

impl<CL, S, Stmt, Predicate> FilterDsl<Predicate> for SelectByStatement<S, Stmt>
where
    Predicate: Expression<SqlType = Bool> + NonAggregate,
    Stmt: FilterDsl<Predicate>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectByStatement::new(self.inner.filter(predicate))
    }
}

impl<CL, S, Stmt, Predicate> OrFilterDsl<Predicate> for SelectByStatement<S, Stmt>
where
    Predicate: Expression<SqlType = Bool> + NonAggregate,
    Stmt: OrFilterDsl<Predicate>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        SelectByStatement::new(self.inner.or_filter(predicate))
    }
}

use crate::query_source::Table;

impl<CL, S, Stmt, PK> FindDsl<PK> for SelectByStatement<S, Stmt>
where
    Stmt: FindDsl<PK>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn find(self, id: PK) -> Self::Output {
        SelectByStatement::new(self.inner.find(id))
    }
}

impl<CL, S, Stmt, Expr> OrderDsl<Expr> for SelectByStatement<S, Stmt>
where
    Expr: Expression,
    Stmt: OrderDsl<Expr>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn order(self, expr: Expr) -> Self::Output {
        SelectByStatement::new(self.inner.order(expr))
    }
}

impl<CL, S, Stmt, Expr> ThenOrderDsl<Expr> for SelectByStatement<S, Stmt>
where
    Expr: Expression,
    Stmt: ThenOrderDsl<Expr>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        SelectByStatement::new(self.inner.then_order_by(expr))
    }
}

impl<CL, S, Stmt> LimitDsl for SelectByStatement<S, Stmt>
where
    Stmt: LimitDsl,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn limit(self, limit: i64) -> Self::Output {
        SelectByStatement::new(self.inner.limit(limit))
    }
}

impl<CL, S, Stmt> OffsetDsl for SelectByStatement<S, Stmt>
where
    Stmt: OffsetDsl,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn offset(self, offset: i64) -> Self::Output {
        SelectByStatement::new(self.inner.offset(offset))
    }
}

impl<CL, S, Stmt, Lock> LockingDsl<Lock> for SelectByStatement<S, Stmt>
where
    Stmt: LockingDsl<Lock>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn with_lock(self, lock: Lock) -> Self::Output {
        SelectByStatement::new(self.inner.with_lock(lock))
    }
}

impl<CL, S, Stmt, Modifier> ModifyLockDsl<Modifier> for SelectByStatement<S, Stmt>
where
    Stmt: ModifyLockDsl<Modifier>,
    Self: SelectByQuery<Columns = CL>,
    Stmt::Output: SelectByQuery<Columns = CL>,
{
    type Output = SelectByStatement<S, Stmt::Output>;

    fn modify_lock(self, modifier: Modifier) -> Self::Output {
        SelectByStatement::new(self.inner.modify_lock(modifier))
    }
}

// impl<'a, F, S, D, W, O, L, Of, G, DB> BoxedDsl<'a, DB>
//     for SelectStatement<F, SelectClause<S>, D, W, O, L, Of, G>
// where
//     Self: AsQuery,
//     DB: Backend,
//     S: QueryFragment<DB> + SelectableExpression<F> + Send + 'a,
//     D: QueryFragment<DB> + Send + 'a,
//     W: Into<BoxedWhereClause<'a, DB>>,
//     O: Into<Option<Box<dyn QueryFragment<DB> + Send + 'a>>>,
//     L: QueryFragment<DB> + Send + 'a,
//     Of: QueryFragment<DB> + Send + 'a,
//     G: QueryFragment<DB> + Send + 'a,
// {
//     type Output = BoxedSelectStatement<'a, S::SqlType, F, DB>;

//     fn internal_into_boxed(self) -> Self::Output {
//         BoxedSelectStatement::new(
//             Box::new(self.select.0),
//             self.from,
//             Box::new(self.distinct),
//             self.where_clause.into(),
//             self.order.into(),
//             Box::new(self.limit),
//             Box::new(self.offset),
//             Box::new(self.group_by),
//         )
//     }
// }

// impl<'a, F, D, W, O, L, Of, G, DB> BoxedDsl<'a, DB>
//     for SelectStatement<F, DefaultSelectClause, D, W, O, L, Of, G>
// where
//     Self: AsQuery,
//     DB: Backend,
//     F: QuerySource,
//     F::DefaultSelection: QueryFragment<DB> + Send + 'a,
//     D: QueryFragment<DB> + Send + 'a,
//     W: Into<BoxedWhereClause<'a, DB>>,
//     O: Into<Option<Box<dyn QueryFragment<DB> + Send + 'a>>>,
//     L: QueryFragment<DB> + Send + 'a,
//     Of: QueryFragment<DB> + Send + 'a,
//     G: QueryFragment<DB> + Send + 'a,
// {
//     type Output = BoxedSelectStatement<'a, <F::DefaultSelection as Expression>::SqlType, F, DB>;
//     fn internal_into_boxed(self) -> Self::Output {
//         BoxedSelectStatement::new(
//             Box::new(self.from.default_selection()),
//             self.from,
//             Box::new(self.distinct),
//             self.where_clause.into(),
//             self.order.into(),
//             Box::new(self.limit),
//             Box::new(self.offset),
//             Box::new(self.group_by),
//         )
//     }
// }

impl<S, Stmt> HasTable for SelectByStatement<S, Stmt>
where
    Stmt: HasTable,
{
    type Table = Stmt::Table;

    fn table() -> Self::Table {
        Stmt::table()
    }
}

// no IntoUpdateTarget: if you'd like to `.into_update_target`, you should not first `.select_by`

// FIXME: Should we disable joining when `.group_by` has been called? Are there
// any other query methods where a join no longer has the same semantics as
// joining on just the table?
impl<S, Stmt, Rhs> JoinTo<Rhs> for SelectByStatement<S, Stmt>
where
    Stmt: JoinTo<Rhs>,
{
    type FromClause = Stmt::FromClause;
    type OnClause = Stmt::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        Stmt::join_target(rhs)
    }
}

impl<S, Stmt> QueryDsl for SelectByStatement<S, Stmt> {}

impl<S, Stmt, Conn> RunQueryDsl<Conn> for SelectByStatement<S, Stmt> {}

// SELECTBY_TODO: Self is never Query
impl<S, Stmt, Tab> Insertable<Tab> for SelectByStatement<S, Stmt>
where
    Tab: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, Tab::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, S, Stmt, Tab> Insertable<Tab> for &'a SelectByStatement<S, Stmt>
where
    Tab: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, Tab::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

// impl<'a, F, S, D, W, O, L, Of, G> SelectNullableDsl
//     for SelectStatement<F, SelectClause<S>, D, W, O, L, Of, G>
// {
//     type Output = SelectStatement<F, SelectClause<Nullable<S>>, D, W, O, L, Of, G>;

//     fn nullable(self) -> Self::Output {
//         SelectStatement::new(
//             SelectClause(Nullable::new(self.select.0)),
//             self.from,
//             self.distinct,
//             self.where_clause,
//             self.order,
//             self.limit,
//             self.offset,
//             self.group_by,
//             self.locking,
//         )
//     }
// }

// impl<'a, F, D, W, O, L, Of, G> SelectNullableDsl
//     for SelectStatement<F, DefaultSelectClause, D, W, O, L, Of, G>
// where
//     F: QuerySource,
// {
//     type Output =
//         SelectStatement<F, SelectClause<Nullable<F::DefaultSelection>>, D, W, O, L, Of, G>;

//     fn nullable(self) -> Self::Output {
//         SelectStatement::new(
//             SelectClause(Nullable::new(self.from.default_selection())),
//             self.from,
//             self.distinct,
//             self.where_clause,
//             self.order,
//             self.limit,
//             self.offset,
//             self.group_by,
//             self.locking,
//         )
//     }
// }
