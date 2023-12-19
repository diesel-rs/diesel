use crate::expression::operators::Concat;
use crate::mysql::backend::MysqlOnConflictClause;
use crate::mysql::Mysql;
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

impl QueryFragment<Mysql> for ForUpdate {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}

impl QueryFragment<Mysql> for ForShare {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" FOR SHARE");
        Ok(())
    }
}

impl QueryFragment<Mysql> for NoModifier {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        Ok(())
    }
}

impl QueryFragment<Mysql> for SkipLocked {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" SKIP LOCKED");
        Ok(())
    }
}

impl QueryFragment<Mysql> for NoWait {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" NOWAIT");
        Ok(())
    }
}

impl QueryFragment<Mysql, crate::mysql::backend::MysqlStyleDefaultValueClause> for DefaultValues {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql("() VALUES ()");
        Ok(())
    }
}

impl<L, R> QueryFragment<Mysql, crate::mysql::backend::MysqlConcatClause> for Concat<L, R>
where
    L: QueryFragment<Mysql>,
    R: QueryFragment<Mysql>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, Mysql>,
    ) -> crate::result::QueryResult<()> {
        out.push_sql("CONCAT(");
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(",");
        self.right.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T> QueryFragment<Mysql, crate::mysql::backend::MysqlOnConflictClause> for DoNothing<T>
where
    T: Table + StaticQueryFragment,
    T::Component: QueryFragment<Mysql>,
    T::PrimaryKey: DoNothingClauseHelper,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" UPDATE ");
        T::PrimaryKey::walk_ast::<T>(out.reborrow())?;
        Ok(())
    }
}

impl<T, Tab> QueryFragment<Mysql, crate::mysql::backend::MysqlOnConflictClause> for DoUpdate<T, Tab>
where
    T: QueryFragment<Mysql>,
    Tab: Table + StaticQueryFragment,
    Tab::PrimaryKey: DoNothingClauseHelper,
    Tab::Component: QueryFragment<Mysql>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
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

impl<Values, Target, Action> QueryFragment<Mysql, MysqlOnConflictClause>
    for OnConflictValues<Values, Target, Action, NoWhereClause>
where
    Values: QueryFragment<Mysql>,
    Target: QueryFragment<Mysql>,
    Action: QueryFragment<Mysql>,
    NoWhereClause: QueryFragment<Mysql>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(" ON DUPLICATE KEY");
        self.target.walk_ast(out.reborrow())?;
        self.action.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out)?;
        Ok(())
    }
}

/// A marker type signaling that the given `ON CONFLICT` clause
/// uses mysql's `ON DUPLICATE KEY` syntax that triggers on
/// all unique constraints
///
/// See [`InsertStatement::on_conflict`](crate::query_builder::InsertStatement::on_conflict)
/// for examples
#[derive(Debug, Copy, Clone)]
pub struct DuplicatedKeys;

impl<Tab> OnConflictTarget<Tab> for ConflictTarget<DuplicatedKeys> {}

impl QueryFragment<Mysql, MysqlOnConflictClause> for ConflictTarget<DuplicatedKeys> {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        Ok(())
    }
}

impl<S> QueryFragment<crate::mysql::Mysql> for OnConflictSelectWrapper<S>
where
    S: QueryFragment<crate::mysql::Mysql>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, crate::mysql::Mysql>) -> QueryResult<()> {
        self.0.walk_ast(out)
    }
}

/// This is a helper trait
/// that provideds a fake `DO NOTHING` clause
/// based on reassigning the possible
/// composite primary key to itself
trait DoNothingClauseHelper {
    fn walk_ast<T>(out: AstPass<'_, '_, Mysql>) -> QueryResult<()>
    where
        T: StaticQueryFragment,
        T::Component: QueryFragment<Mysql>;
}

impl<C> DoNothingClauseHelper for C
where
    C: Column,
{
    fn walk_ast<T>(mut out: AstPass<'_, '_, Mysql>) -> QueryResult<()>
    where
        T: StaticQueryFragment,
        T::Component: QueryFragment<Mysql>,
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
            impl<$($T,)*> DoNothingClauseHelper for ($($T,)*)
            where $($T: Column,)*
            {
                fn walk_ast<Table>(mut out: AstPass<'_, '_, Mysql>) -> QueryResult<()>
                where
                    Table: StaticQueryFragment,
                    Table::Component: QueryFragment<Mysql>,
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

diesel_derives::__diesel_for_each_tuple!(do_nothing_for_composite_keys);
