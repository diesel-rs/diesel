use crate::expression::TypedExpressionType;
use crate::expression::ValidGrouping;
use crate::query_builder::AsQuery;
use crate::query_builder::FromClause;
use crate::query_builder::SelectStatement;
use crate::query_source::Table;
use crate::Expression;

/// Methods related to locking select statements
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_update` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait LockingDsl<Lock> {
    /// The type returned by `set_lock`. See [`dsl::ForUpdate`] and friends for
    /// convenient access to this type.
    ///
    /// [`dsl::ForUpdate`]: crate::dsl::ForUpdate
    type Output;

    /// See the trait level documentation
    fn with_lock(self, lock: Lock) -> Self::Output;
}

impl<T, Lock> LockingDsl<Lock> for T
where
    T: Table + AsQuery<Query = SelectStatement<FromClause<T>>>,
    T::DefaultSelection: Expression<SqlType = T::SqlType> + ValidGrouping<()>,
    T::SqlType: TypedExpressionType,
{
    type Output = <SelectStatement<FromClause<T>> as LockingDsl<Lock>>::Output;

    fn with_lock(self, lock: Lock) -> Self::Output {
        self.as_query().with_lock(lock)
    }
}

/// Methods related to modifiers on locking select statements
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `skip_locked` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait ModifyLockDsl<Modifier> {
    /// The type returned by `modify_lock`. See [`dsl::SkipLocked`] and friends
    /// for convenient access to this type.
    ///
    /// [`dsl::SkipLocked`]: crate::dsl::SkipLocked
    type Output;

    /// See the trait level documentation
    fn modify_lock(self, modifier: Modifier) -> Self::Output;
}
