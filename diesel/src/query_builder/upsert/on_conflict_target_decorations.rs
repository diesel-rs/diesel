use crate::backend::Backend;
use crate::expression::Expression;
use crate::query_builder::upsert::on_conflict_target::{ConflictTarget, NoConflictTarget};
use crate::query_builder::where_clause::{NoWhereClause, WhereAnd, WhereClause};
use crate::query_builder::{AstPass, QueryFragment, QueryResult};
use crate::sql_types::BoolOrNullableBool;

pub trait UndecoratedConflictTarget {}

impl UndecoratedConflictTarget for NoConflictTarget {}
impl<T> UndecoratedConflictTarget for ConflictTarget<T> {}

/// Adds a `WHERE` predicate to an `ON CONFLICT` target.
///
/// This enables the `ON CONFLICT (target) WHERE predicate DO ...` SQL syntax
/// on PostgreSQL. PostgreSQL uses the predicate to select which unique index
/// to match against. Any unique index whose `WHERE` clause is implied by
/// the predicate qualifies.
///
/// Calling `.filter_target()` multiple times combines the predicates with `AND`.
pub trait DecoratableTarget<P> {
    /// The type returned by [`filter_target`](DecoratableTarget::filter_target).
    type FilterOutput;
    /// Adds a `WHERE` predicate to the `ON CONFLICT` target, telling PostgreSQL
    /// which unique index to check for conflicts (PostgreSQL only).
    ///
    /// This generates `ON CONFLICT (target) WHERE predicate DO ...` SQL.
    /// PostgreSQL selects unique indexes whose `WHERE` clause is implied by
    /// the predicate; an exact match is not required.
    ///
    /// Calling `.filter_target()` multiple times combines predicates with `AND`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// diesel::insert_into(users)
    ///     .values(&new_user)
    ///     .on_conflict(name)
    ///     .filter_target(id.gt(5))
    ///     .do_nothing()
    ///     .execute(conn)?;
    /// ```
    ///
    /// For full runnable examples including `do_update` and predicate chaining,
    /// see [`IncompleteOnConflict`]'s implementation of this trait.
    ///
    /// [`IncompleteOnConflict`]: crate::upsert::IncompleteOnConflict
    fn filter_target(self, predicate: P) -> Self::FilterOutput;
}

#[derive(Debug)]
pub struct DecoratedConflictTarget<T, U> {
    pub(crate) target: T,
    pub(crate) where_clause: U,
}

impl<T, P> DecoratableTarget<P> for T
where
    P: Expression,
    P::SqlType: BoolOrNullableBool,
    T: UndecoratedConflictTarget,
{
    type FilterOutput = DecoratedConflictTarget<T, WhereClause<P>>;

    fn filter_target(self, predicate: P) -> Self::FilterOutput {
        DecoratedConflictTarget {
            target: self,
            where_clause: NoWhereClause.and(predicate),
        }
    }
}

impl<T, U, P> DecoratableTarget<P> for DecoratedConflictTarget<T, U>
where
    P: Expression,
    P::SqlType: BoolOrNullableBool,
    U: WhereAnd<P>,
{
    type FilterOutput = DecoratedConflictTarget<T, <U as WhereAnd<P>>::Output>;

    fn filter_target(self, predicate: P) -> Self::FilterOutput {
        DecoratedConflictTarget {
            target: self.target,
            where_clause: self.where_clause.and(predicate),
        }
    }
}

impl<DB, T, U> QueryFragment<DB> for DecoratedConflictTarget<T, U>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}
