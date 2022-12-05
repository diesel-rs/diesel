use std::fmt::Debug;

use crate::backend::{Backend, DieselReserveSpecialization};
use crate::query_builder::where_clause::{NoWhereClause, WhereAnd};
use crate::query_builder::{AsQuery, AstPass, QueryFragment, QueryId};
use crate::query_dsl::filter_dsl::FilterDsl;
use crate::{QueryResult, RunQueryDsl};

#[derive(Debug, Clone, Copy)]
pub struct ConditionallyInsertStatement<Stmt: Copy, WhereClause: Copy = NoWhereClause> {
    insert: Stmt,
    where_clause: WhereClause,
}

impl<Stmt: Copy> ConditionallyInsertStatement<Stmt> {
    pub fn new(insert: Stmt) -> ConditionallyInsertStatement<Stmt> {
        ConditionallyInsertStatement::<Stmt> {
            insert,
            where_clause: NoWhereClause {},
        }
    }
}

impl<Stmt, WhereClause> QueryId for ConditionallyInsertStatement<Stmt, WhereClause>
where
    Stmt: Copy + QueryId,
    WhereClause: Copy,
{
    type QueryId = Stmt::QueryId;
    const HAS_STATIC_QUERY_ID: bool = Stmt::HAS_STATIC_QUERY_ID;
}

impl<Stmt, WhereClause, DB> QueryFragment<DB> for ConditionallyInsertStatement<Stmt, WhereClause>
where
    DB: Backend + DieselReserveSpecialization,
    Stmt: Copy + QueryFragment<DB>,
    WhereClause: Copy + QueryFragment<DB>,
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
    Stmt: Copy + AsQuery,
    WhereClause: Copy,
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
    Stmt: Copy + AsQuery,
    WhereClause: Copy + WhereAnd<Predicate>,
    WhereClause::Output: Copy,
{
    type Output = ConditionallyInsertStatement<Stmt, WhereClause::Output>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        ConditionallyInsertStatement {
            insert: self.insert,
            where_clause: self.where_clause.and(predicate),
        }
    }
}

impl<Stmt, WhereClause, Conn> RunQueryDsl<Conn> for ConditionallyInsertStatement<Stmt, WhereClause>
where
    Stmt: Copy + QueryId + AsQuery,
    WhereClause: Copy,
{
}
