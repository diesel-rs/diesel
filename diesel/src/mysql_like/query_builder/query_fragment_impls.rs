use crate::backend::Backend;
use crate::expression::operators::Concat;
use crate::mysql_like::MysqlLikeBackend;
use crate::query_builder::insert_statement::DefaultValues;
use crate::query_builder::locking_clause::{ForShare, ForUpdate, NoModifier, NoWait, SkipLocked};
use crate::query_builder::nodes::StaticQueryFragment;
use crate::query_builder::upsert::into_conflict_clause::OnConflictSelectWrapper;
use crate::query_builder::upsert::on_conflict_actions::{DoNothing, DoUpdate, Excluded};
use crate::query_builder::upsert::on_conflict_clause::OnConflictValues;
use crate::query_builder::upsert::on_conflict_target::{ConflictTarget, OnConflictTarget};
use crate::query_builder::where_clause::NoWhereClause;
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::QueryResult;
use crate::{Column, Table};

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend> QueryFragment<DB> for ForUpdate {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend> QueryFragment<DB> for ForShare {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" FOR SHARE");
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend> QueryFragment<DB> for NoModifier {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend> QueryFragment<DB> for SkipLocked {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" SKIP LOCKED");
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend> QueryFragment<DB> for NoWait {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" NOWAIT");
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend>
    QueryFragment<DB, crate::mysql_like::query_fragments::MysqlStyleDefaultValueClause>
    for DefaultValues
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("() VALUES ()");
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend, L, R>
    QueryFragment<DB, crate::mysql_like::query_fragments::MysqlConcatClause> for Concat<L, R>
where
    L: QueryFragment<DB>,
    R: QueryFragment<DB>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::result::QueryResult<()> {
        out.push_sql("CONCAT(");
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(",");
        self.right.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend, T>
    QueryFragment<DB, crate::mysql_like::query_fragments::MysqlOnConflictClause> for DoNothing<T>
where
    T: Table + StaticQueryFragment,
    T::Component: QueryFragment<DB>,
    T::PrimaryKey: DoNothingClauseHelper<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" UPDATE ");
        T::PrimaryKey::walk_ast::<T>(out.reborrow())?;
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend, T, Tab>
    QueryFragment<DB, crate::mysql_like::query_fragments::MysqlOnConflictClause>
    for DoUpdate<T, Tab>
where
    T: QueryFragment<DB>,
    Tab: Table + StaticQueryFragment,
    Tab::PrimaryKey: DoNothingClauseHelper<DB>,
    Tab::Component: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        out.push_sql(" UPDATE ");
        if self.changeset.is_noop(out.backend())? {
            Tab::PrimaryKey::walk_ast::<Tab>(out.reborrow())?;
        } else {
            self.changeset.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend, Values, Target, Action>
    QueryFragment<DB, crate::mysql_like::query_fragments::MysqlOnConflictClause>
    for OnConflictValues<Values, Target, Action, NoWhereClause>
where
    Values: QueryFragment<DB>,
    Target: QueryFragment<DB>,
    Action: QueryFragment<DB>,
    NoWhereClause: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(" ON DUPLICATE KEY");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out)?;
        Ok(())
    }
}

impl<DB, T> QueryFragment<DB, crate::mysql_like::query_fragments::MysqlOnConflictClause>
    for Excluded<T>
where
    DB: Backend<OnConflictClause = crate::mysql_like::query_fragments::MysqlOnConflictClause>,
    T: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("VALUES(");
        out.push_identifier(T::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

/// A marker type signaling that the given `ON CONFLICT` clause
/// uses Mysql's `ON DUPLICATE KEY` syntax that triggers on
/// all unique constraints
///
/// See [`InsertStatement::on_conflict`](crate::query_builder::InsertStatement::on_conflict)
/// for examples
#[derive(Debug, Copy, Clone)]
pub struct DuplicatedKeys;

impl<Tab> OnConflictTarget<Tab> for ConflictTarget<DuplicatedKeys> {}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend>
    QueryFragment<DB, crate::mysql_like::query_fragments::MysqlOnConflictClause>
    for ConflictTarget<DuplicatedKeys>
{
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend, S> QueryFragment<DB> for OnConflictSelectWrapper<S>
where
    S: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(out)
    }
}

/// This is a helper trait
/// that provides a fake `DO NOTHING` clause
/// based on reassigning the possible
/// composite primary key to itself
trait DoNothingClauseHelper<DB: MysqlLikeBackend> {
    fn walk_ast<T>(out: AstPass<'_, '_, DB>) -> QueryResult<()>
    where
        T: StaticQueryFragment,
        T::Component: QueryFragment<DB>;
}

#[diagnostic::do_not_recommend]
impl<DB: MysqlLikeBackend, C> DoNothingClauseHelper<DB> for C
where
    C: Column,
{
    fn walk_ast<T>(mut out: AstPass<'_, '_, DB>) -> QueryResult<()>
    where
        T: StaticQueryFragment,
        T::Component: QueryFragment<DB>,
    {
        T::STATIC_COMPONENT.walk_ast(out.reborrow())?;
        out.push_sql(".");
        out.push_identifier(C::NAME)?;
        out.push_sql(" = ");
        T::STATIC_COMPONENT.walk_ast(out.reborrow())?;
        out.push_sql(".");
        out.push_identifier(C::NAME)?;
        Ok(())
    }
}

macro_rules! do_nothing_for_composite_keys {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<DB: MysqlLikeBackend,$($T,)*> DoNothingClauseHelper<DB> for ($($T,)*)
            where $($T: Column,)*
            {
                fn walk_ast<Table>(mut out: AstPass<'_, '_, DB>) -> QueryResult<()>
                where
                    Table: StaticQueryFragment,
                    Table::Component: QueryFragment<DB>,
                {
                    let mut first = true;
                    $(
                        #[allow(unused_assignments)]
                        if first {
                            first = false;
                        } else {
                            out.push_sql(", ");
                        }
                        Table::STATIC_COMPONENT.walk_ast(out.reborrow())?;
                        out.push_sql(".");
                        out.push_identifier($T::NAME)?;
                        out.push_sql(" = ");
                        Table::STATIC_COMPONENT.walk_ast(out.reborrow())?;
                        out.push_sql(".");
                        out.push_identifier($T::NAME)?;
                    )*
                    Ok(())
                }
            }
        )*
    }
}

crate::for_each_tuple!(do_nothing_for_composite_keys);
