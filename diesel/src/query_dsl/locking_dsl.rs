use query_builder::AsQuery;
use query_source::Table;

/// The `for_update` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_update` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait ForUpdateDsl {
    /// The type returned by `for_update`. See [`dsl::ForUpdate`] for
    /// convenient access to this type.
    ///
    /// [`dsl::ForUpdate`]: ../../dsl/type.ForUpdate.html
    type Output;

    /// See the trait level documentation
    fn for_update(self) -> Self::Output;
}

impl<T> ForUpdateDsl for T
where
    T: Table + AsQuery,
    T::Query: ForUpdateDsl,
{
    type Output = <T::Query as ForUpdateDsl>::Output;

    fn for_update(self) -> Self::Output {
        self.as_query().for_update()
    }
}

/// The `for_no_key_update` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_no_key_update` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait ForNoKeyUpdateDsl {
    /// The type returned by `for_no_key_update`. See [`dsl::ForNoKeyUpdate`] for
    /// convenient access to this type.
    ///
    /// [`dsl::ForNoKeyUpdate`]: ../../dsl/type.ForNoKeyUpdate.html
    type Output;

    /// See the trait level documentation
    fn for_no_key_update(self) -> Self::Output;
}

impl<T> ForNoKeyUpdateDsl for T
where
    T: Table + AsQuery,
    T::Query: ForNoKeyUpdateDsl,
{
    type Output = <T::Query as ForNoKeyUpdateDsl>::Output;

    fn for_no_key_update(self) -> Self::Output {
        self.as_query().for_no_key_update()
    }
}

/// The `for_share` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_share` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait ForShareDsl {
    /// The type returned by `for_share`. See [`dsl::ForShare`] for
    /// convenient access to this type.
    ///
    /// [`dsl::ForShare`]: ../../dsl/type.ForShare.html
    type Output;

    /// See the trait level documentation
    fn for_share(self) -> Self::Output;
}

impl<T> ForShareDsl for T
where
    T: Table + AsQuery,
    T::Query: ForShareDsl,
{
    type Output = <T::Query as ForShareDsl>::Output;

    fn for_share(self) -> Self::Output {
        self.as_query().for_share()
    }
}

/// The `for_key_share` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_key_share` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait ForKeyShareDsl {
    /// The type returned by `for_key_share`. See [`dsl::ForKeyShare`] for
    /// convenient access to this type.
    ///
    /// [`dsl::ForKeyShare`]: ../../dsl/type.ForKeyShare.html
    type Output;

    /// See the trait level documentation
    fn for_key_share(self) -> Self::Output;
}

impl<T> ForKeyShareDsl for T
where
    T: Table + AsQuery,
    T::Query: ForKeyShareDsl,
{
    type Output = <T::Query as ForKeyShareDsl>::Output;

    fn for_key_share(self) -> Self::Output {
        self.as_query().for_key_share()
    }
}

/// The `skip_locked` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `skip_locked` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait SkipLockedDsl {
    /// The type returned by `skip_locked`. See [`dsl::SkipLocked`] for
    /// convenient access to this type.
    ///
    /// [`dsl::SkipLocked`]: ../../dsl/type.SkipLocked.html
    type Output;

    /// See the trait level documentation
    fn skip_locked(self) -> Self::Output;
}

/// The `no_wait` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `no_wait` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait NoWaitDsl {
    /// The type returned by `no_wait`. See [`dsl::NoWait`] for
    /// convenient access to this type.
    ///
    /// [`dsl::NoWait`]: ../../dsl/type.NoWait.html
    type Output;

    /// See the trait level documentation
    fn no_wait(self) -> Self::Output;
}
