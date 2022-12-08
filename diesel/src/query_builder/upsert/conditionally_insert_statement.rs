use std::fmt::Debug;

use crate::backend::{Backend, DieselReserveSpecialization};
use crate::query_builder::upsert::on_conflict_clause::OnConflictValues;
use crate::query_builder::where_clause::{NoWhereClause, WhereAnd};
use crate::query_builder::{
    where_clause, AsQuery, AstPass, InsertStatement, QueryFragment, QueryId,
};
use crate::query_dsl::filter_dsl::FilterDsl;
use crate::sql_types::BoolOrNullableBool;
use crate::{Expression, QueryResult, QuerySource, RunQueryDsl};

#[derive(Debug, Clone)]
pub struct ConditionallyInsertStatement<Stmt, WhereClause = NoWhereClause> {
    insert: Stmt,
    where_clause: WhereClause,
}

impl<Stmt, WhereClause> ConditionallyInsertStatement<Stmt, WhereClause> {
    pub fn new(
        insert: Stmt,
        where_clause: WhereClause,
    ) -> ConditionallyInsertStatement<Stmt, WhereClause> {
        ConditionallyInsertStatement {
            insert,
            where_clause,
        }
    }
}

impl<Stmt, WhereClause> QueryId for ConditionallyInsertStatement<Stmt, WhereClause>
where
    Stmt: QueryId,
{
    type QueryId = Stmt::QueryId;
    const HAS_STATIC_QUERY_ID: bool = Stmt::HAS_STATIC_QUERY_ID;
}

impl<Stmt, WhereClause, DB> QueryFragment<DB> for ConditionallyInsertStatement<Stmt, WhereClause>
where
    DB: Backend + DieselReserveSpecialization,
    Stmt: QueryFragment<DB>,
    WhereClause: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.insert.walk_ast(out.reborrow())?;
        out.push_sql(" ");
        self.where_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Stmt, WhereClause> AsQuery for ConditionallyInsertStatement<Stmt, WhereClause>
where
    Stmt: AsQuery,
{
    type SqlType = Stmt::SqlType;
    type Query = Stmt::Query;

    fn as_query(self) -> Self::Query {
        Stmt::as_query(self.insert)
    }
}

impl<Stmt, WhereClause, Predicate> FilterDsl<Predicate>
    for ConditionallyInsertStatement<Stmt, WhereClause>
where
    Stmt: AsQuery,
    WhereClause: WhereAnd<Predicate>,
{
    type Output = ConditionallyInsertStatement<Stmt, WhereClause::Output>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        ConditionallyInsertStatement {
            insert: self.insert,
            where_clause: self.where_clause.and(predicate),
        }
    }
}

impl<Stmt, WhereClause, Conn> RunQueryDsl<Conn> for ConditionallyInsertStatement<Stmt, WhereClause> where
    Stmt: QueryId + AsQuery
{
}

impl<Stmt: Copy, WhereClause: Copy> Copy for ConditionallyInsertStatement<Stmt, WhereClause> {}

trait ConditionallyInsertable<T: QuerySource> {}

impl<T, U, Target, Action> ConditionallyInsertable<T> for OnConflictValues<U, Target, Action> where
    T: QuerySource
{
}

impl<T: QuerySource, U: ConditionallyInsertable<T>, Op, Ret, Predicate> FilterDsl<Predicate>
    for InsertStatement<T, U, Op, Ret>
where
    T: QuerySource,
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = ConditionallyInsertStatement<Self, where_clause::WhereClause<Predicate>>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        let where_clause = NoWhereClause {};
        ConditionallyInsertStatement::new(self, where_clause.and(predicate))
    }
}
