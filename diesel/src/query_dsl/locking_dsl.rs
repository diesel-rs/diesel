use query_builder::AsQuery;
use query_source::Table;

/// Methods related to locking select statements
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_update` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait LockingDsl<Lock> {
    /// The type returned by `set_lock`. See [`dsl::ForUpdate`] and friends for
    /// convenient access to this type.
    ///
    /// [`dsl::ForUpdate`]: ../../dsl/type.ForUpdate.html
    type Output;

    /// See the trait level documentation
    fn with_lock(self, lock: Lock) -> Self::Output;
}

impl<T, Lock> LockingDsl<Lock> for T
where
    T: Table + AsQuery,
    T::Query: LockingDsl<Lock>,
{
    type Output = <T::Query as LockingDsl<Lock>>::Output;

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
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait ModifyLockDsl<Modifier> {
    /// The type returned by `modify_lock`. See [`dsl::SkipLocked`] and friends
    /// for convenient access to this type.
    ///
    /// [`dsl::SkipLocked`]: ../../dsl/type.SkipLocked.html
    type Output;

    /// See the trait level documentation
    fn modify_lock(self, modifier: Modifier) -> Self::Output;
}
