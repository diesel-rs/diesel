/// Treats tuples as a list which can be appended to. e.g.
/// `(a,).tuple_append(b) == (a, b)`
pub trait TupleAppend<T> {
    type Output;

    fn tuple_append(self, right: T) -> Self::Output;
}

#[cfg(not(feature = "unstable"))]
mod polyfill {
    /// A polyfill for `std::ptr::NonNull`, which was stabilized in Rust 1.25.
    /// When our minimum supported version of Rust is >= 1.25, this should be
    /// removed. However, we should not bump our minimum version of Rust just
    /// to remove this polyfill.
    pub(crate) struct NonNull<T: ?Sized> {
        ptr: *mut T,
    }

    impl<T: ?Sized> Clone for NonNull<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T: ?Sized> Copy for NonNull<T> {}

    impl<T: ?Sized> NonNull<T> {
        pub(crate) unsafe fn new_unchecked(ptr: *mut T) -> Self {
            Self { ptr }
        }

        pub(crate) fn new(ptr: *mut T) -> Option<Self> {
            if ptr.is_null() {
                None
            } else {
                Some(unsafe { Self::new_unchecked(ptr) })
            }
        }

        pub(crate) fn as_ptr(self) -> *mut T {
            self.ptr
        }
    }
}

#[cfg(not(feature = "unstable"))]
pub(crate) use self::polyfill::*;

#[cfg(feature = "unstable")]
pub(crate) type NonNull<T> = ::std::ptr::NonNull<T>;
