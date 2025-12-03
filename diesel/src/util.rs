/// Treats tuples as a list which can be appended to. e.g.
/// `(a,).tuple_append(b) == (a, b)`
pub trait TupleAppend<T> {
    type Output;

    fn tuple_append(self, right: T) -> Self::Output;
}

pub trait TupleSize {
    const SIZE: usize;
}

#[cfg(not(feature = "std"))]
pub(crate) mod std_compat {
    pub(crate) type Entry<'a, K, V> =
        hashbrown::hash_map::Entry<'a, K, V, hashbrown::DefaultHashBuilder>;
    pub(crate) use hashbrown::HashMap;

    pub(crate) fn catch_unwind<R>(f: impl FnOnce() -> R) -> Result<R, ()> {
        Ok(f())
    }
    pub(crate) fn panicking() -> bool {
        false
    }

    pub(crate) fn abort() -> ! {
        struct DropBomb;

        impl Drop for DropBomb {
            fn drop(&mut self) {
                panic!("Abort");
            }
        }

        let _guard = DropBomb;

        panic!("Abort");
    }
}

#[cfg(feature = "std")]
pub(crate) mod std_compat {
    pub(crate) use std::collections::HashMap;
    pub(crate) use std::collections::hash_map::Entry;
    #[cfg(feature = "__sqlite-shared")]
    pub(crate) use std::panic::catch_unwind;
    #[cfg(feature = "__sqlite-shared")]
    pub(crate) use std::process::abort;
    #[cfg(feature = "__sqlite-shared")]
    pub(crate) use std::thread::panicking;
}
