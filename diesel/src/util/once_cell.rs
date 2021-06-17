// This is a copy of the unstable `OnceCell` implementation in rusts std-library
// https://github.com/rust-lang/rust/blob/1160cf864f2a0014e3442367e1b96496bfbeadf4/library/core/src/lazy.rs#L8-L276
//
// See https://github.com/rust-lang/rust/issues/74465 for the corresponding tracking issue

use std::cell::UnsafeCell;

/// A cell which can be written to only once.
///
/// Unlike `RefCell`, a `OnceCell` only provides shared `&T` references to its value.
/// Unlike `Cell`, a `OnceCell` doesn't require copying or replacing the value to access it.
///
/// # Examples
///
/// ```
/// #![feature(once_cell)]
///
/// use std::lazy::OnceCell;
///
/// let cell = OnceCell::new();
/// assert!(cell.get().is_none());
///
/// let value: &String = cell.get_or_init(|| {
///     "Hello, World!".to_string()
/// });
/// assert_eq!(value, "Hello, World!");
/// assert!(cell.get().is_some());
/// ```
pub struct OnceCell<T> {
    // Invariant: written to at most once.
    inner: UnsafeCell<Option<T>>,
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OnceCell<T> {
    /// Creates a new empty cell.
    pub const fn new() -> OnceCell<T> {
        OnceCell {
            inner: UnsafeCell::new(None),
        }
    }

    /// Gets the contents of the cell, initializing it with `f` if
    /// the cell was empty. If the cell was empty and `f` failed, an
    /// error is returned.
    ///
    /// # Panics
    ///
    /// If `f` panics, the panic is propagated to the caller, and the cell
    /// remains uninitialized.
    ///
    /// It is an error to reentrantly initialize the cell from `f`. Doing
    /// so results in a panic.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(once_cell)]
    ///
    /// use std::lazy::OnceCell;
    ///
    /// let cell = OnceCell::new();
    /// assert_eq!(cell.get_or_try_init(|| Err(())), Err(()));
    /// assert!(cell.get().is_none());
    /// let value = cell.get_or_try_init(|| -> Result<i32, ()> {
    ///     Ok(92)
    /// });
    /// assert_eq!(value, Ok(&92));
    /// assert_eq!(cell.get(), Some(&92))
    /// ```
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(val) = self.get() {
            return val;
        }
        let val = f();
        // Note that *some* forms of reentrant initialization might lead to
        // UB (see `reentrant_init` test). I believe that just removing this
        // `assert`, while keeping `set/get` would be sound, but it seems
        // better to panic, rather than to silently use an old value.
        assert!(self.set(val).is_ok(), "reentrant init");
        self.get().unwrap()
    }

    pub(crate) fn get(&self) -> Option<&T> {
        // SAFETY: Safe due to `inner`'s invariant
        unsafe { &*self.inner.get() }.as_ref()
    }

    pub(crate) fn set(&self, value: T) -> Result<(), T> {
        // SAFETY: Safe because we cannot have overlapping mutable borrows
        let slot = unsafe { &*self.inner.get() };
        if slot.is_some() {
            return Err(value);
        }

        // SAFETY: This is the only place where we set the slot, no races
        // due to reentrancy/concurrency are possible, and we've
        // checked that slot is currently `None`, so this write
        // maintains the `inner`'s invariant.
        let slot = unsafe { &mut *self.inner.get() };
        *slot = Some(value);
        Ok(())
    }
}
