use crate::expression::operators::Concat;
use crate::mysql_like::MysqlLikeBackend;
use crate::query_builder::insert_statement::DefaultValues;
use crate::query_builder::locking_clause::{ForShare, ForUpdate, NoModifier, NoWait, SkipLocked};
use crate::query_builder::nodes::StaticQueryFragment;
use crate::query_builder::upsert::into_conflict_clause::OnConflictSelectWrapper;
use crate::query_builder::upsert::on_conflict_actions::{DoNothing, DoUpdate};
use crate::query_builder::upsert::on_conflict_clause::OnConflictValues;
use crate::query_builder::upsert::on_conflict_target::{ConflictTarget, OnConflictTarget};
use crate::query_builder::where_clause::NoWhereClause;
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::QueryResult;
use crate::{Column, Table};

impl<B: MysqlLikeBackend> QueryFragment<B> for ForUpdate {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}

impl<B: MysqlLikeBackend> QueryFragment<B> for ForShare {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        out.push_sql(" FOR SHARE");
        Ok(())
    }
}

impl<B: MysqlLikeBackend> QueryFragment<B> for NoModifier {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        Ok(())
    }
}

impl<B: MysqlLikeBackend> QueryFragment<B> for SkipLocked {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        out.push_sql(" SKIP LOCKED");
        Ok(())
    }
}

impl<B: MysqlLikeBackend> QueryFragment<B> for NoWait {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        out.push_sql(" NOWAIT");
        Ok(())
    }
}

impl<B: MysqlLikeBackend>
    QueryFragment<B, crate::mysql_like::query_fragments::MysqlStyleDefaultValueClause>
    for DefaultValues
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        out.push_sql("() VALUES ()");
        Ok(())
    }
}

impl<B: MysqlLikeBackend, L, R>
    QueryFragment<B, crate::mysql_like::query_fragments::MysqlConcatClause> for Concat<L, R>
where
    L: QueryFragment<B>,
    R: QueryFragment<B>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, B>,
    ) -> crate::result::QueryResult<()> {
        out.push_sql("CONCAT(");
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(",");
        self.right.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<B: MysqlLikeBackend, T>
    QueryFragment<B, crate::mysql_like::query_fragments::MysqlOnConflictClause> for DoNothing<T>
where
    T: Table + StaticQueryFragment,
    T::Component: QueryFragment<B>,
    T::PrimaryKey: DoNothingClauseHelper<B>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        out.push_sql(" UPDATE ");
        T::PrimaryKey::walk_ast::<T>(out.reborrow())?;
        Ok(())
    }
}

impl<B: MysqlLikeBackend, T, Tab>
    QueryFragment<B, crate::mysql_like::query_fragments::MysqlOnConflictClause> for DoUpdate<T, Tab>
where
    T: QueryFragment<B>,
    Tab: Table + StaticQueryFragment,
    Tab::PrimaryKey: DoNothingClauseHelper<B>,
    Tab::Component: QueryFragment<B>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
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

impl<B: MysqlLikeBackend, Values, Target, Action>
    QueryFragment<B, crate::mysql_like::query_fragments::MysqlOnConflictClause>
    for OnConflictValues<Values, Target, Action, NoWhereClause>
where
    Values: QueryFragment<B>,
    Target: QueryFragment<B>,
    Action: QueryFragment<B>,
    NoWhereClause: QueryFragment<B>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(" ON DUPLICATE KEY");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out)?;
        Ok(())
    }
}

/// A marker type signaling that the given `ON CONFLICT` clause
/// uses B's `ON DUPLICATE KEY` syntax that triggers on
/// all unique constraints
///
/// See [`InsertStatement::on_conflict`](crate::query_builder::InsertStatement::on_conflict)
/// for examples
#[derive(Debug, Copy, Clone)]
pub struct DuplicatedKeys;

impl<Tab> OnConflictTarget<Tab> for ConflictTarget<DuplicatedKeys> {}

impl<B: MysqlLikeBackend>
    QueryFragment<B, crate::mysql_like::query_fragments::MysqlOnConflictClause>
    for ConflictTarget<DuplicatedKeys>
{
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        Ok(())
    }
}

impl<B: MysqlLikeBackend, S> QueryFragment<B> for OnConflictSelectWrapper<S>
where
    S: QueryFragment<B>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, B>) -> QueryResult<()> {
        self.0.walk_ast(out)
    }
}

/// This is a helper trait
/// that provides a fake `DO NOTHING` clause
/// based on reassigning the possible
/// composite primary key to itself
trait DoNothingClauseHelper<B: MysqlLikeBackend> {
    fn walk_ast<T>(out: AstPass<'_, '_, B>) -> QueryResult<()>
    where
        T: StaticQueryFragment,
        T::Component: QueryFragment<B>;
}

impl<B: MysqlLikeBackend, C> DoNothingClauseHelper<B> for C
where
    C: Column,
{
    fn walk_ast<T>(mut out: AstPass<'_, '_, B>) -> QueryResult<()>
    where
        T: StaticQueryFragment,
        T::Component: QueryFragment<B>,
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
            impl<B: MysqlLikeBackend,$($T,)*> DoNothingClauseHelper<B> for ($($T,)*)
            where $($T: Column,)*
            {
                fn walk_ast<Table>(mut out: AstPass<'_, '_, B>) -> QueryResult<()>
                where
                    Table: StaticQueryFragment,
                    Table::Component: QueryFragment<B>,
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
