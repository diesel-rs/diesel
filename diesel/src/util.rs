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
    #[cfg(feature = "hashbrown")]
    pub(crate) type Entry<'a, K, V> =
        hashbrown::hash_map::Entry<'a, K, V, hashbrown::DefaultHashBuilder>;
    #[cfg(feature = "hashbrown")]
    pub(crate) use hashbrown::HashMap;

    // a dummy map impl that provides
    // the absulut minimum of api that is required to
    // compile diesel without any features compiled
    //
    // This won't be actually used by anyone as any practical
    // usage will rely on the `std` or `sqlite-nostd` (which pulls in hashbrown)
    // feature instead
    //
    // The only way you actually get this with the public API
    // is via the StatementCache implementation
    // for the case that you only enabled `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
    // so it might be fine to require those users to either choose `std` or `hashbrown` as feature
    #[cfg(not(feature = "hashbrown"))]
    mod dummy_map {
        use core::borrow::Borrow;

        pub(crate) struct VecMap<K, V>(alloc::vec::Vec<(K, V)>);

        impl<K, V> Default for VecMap<K, V> {
            fn default() -> Self {
                Self(alloc::vec::Vec::new())
            }
        }

        impl<K, V> FromIterator<(K, V)> for VecMap<K, V> {
            fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
                Self(iter.into_iter().collect())
            }
        }

        impl<K, V> VecMap<K, V> {
            pub(crate) fn get<Q>(&self, key: &Q) -> Option<&V>
            where
                K: Borrow<Q>,
                Q: PartialEq,
            {
                self.0
                    .iter()
                    .find_map(|(k, v)| (k.borrow() == key).then_some(v))
            }

            pub(crate) fn entry(&mut self, key: K) -> Entry<'_, K, V>
            where
                K: PartialEq,
            {
                // We need to use the index based lookup here to workaround borrowck
                // limitations
                // FIXME: polonius
                if let Some(pos) = self.0.iter().position(|(k, _)| *k == key) {
                    Entry::Occupied(OccupiedEntry {
                        entry: &mut self.0[pos],
                    })
                } else {
                    Entry::Vacant(VacantEntry {
                        vec: &mut self.0,
                        key,
                    })
                }
            }
        }

        #[derive(Debug)]
        pub struct OccupiedEntry<'a, K, V> {
            entry: &'a mut (K, V),
        }

        impl<'a, K, V> OccupiedEntry<'a, K, V> {
            pub(crate) fn into_mut(self) -> &'a mut V {
                &mut self.entry.1
            }
        }

        #[derive(Debug)]
        pub struct VacantEntry<'a, K, V> {
            vec: &'a mut alloc::vec::Vec<(K, V)>,
            key: K,
        }

        impl<'a, K, V> VacantEntry<'a, K, V> {
            pub(crate) fn key(&self) -> &K {
                &self.key
            }

            pub(crate) fn insert(self, value: V) -> &'a mut V {
                let Self { vec, key } = self;
                vec.push((key, value));
                &mut vec.last_mut().expect("We literally pushed it above").1
            }
        }

        #[derive(Debug)]
        pub enum Entry<'a, K, V> {
            Occupied(OccupiedEntry<'a, K, V>),
            Vacant(VacantEntry<'a, K, V>),
        }
    }

    #[cfg(not(feature = "hashbrown"))]
    pub(crate) use self::dummy_map::VecMap as HashMap;
    #[cfg(not(feature = "hashbrown"))]
    pub(crate) type Entry<'a, K, V> = self::dummy_map::Entry<'a, K, V>;

    #[cfg(feature = "__sqlite-shared")]
    pub(crate) fn catch_unwind<R>(f: impl FnOnce() -> R) -> Result<R, ()> {
        Ok(f())
    }

    #[cfg(feature = "__sqlite-shared")]
    pub(crate) fn panicking() -> bool {
        false
    }

    #[cfg(feature = "__sqlite-shared")]
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
    // Used only by the statement cache, which needs a connection, not a bare backend.
    #[cfg(any(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        feature = "__sqlite-shared",
        feature = "mysql",
        feature = "mariadb",
        feature = "postgres"
    ))]
    pub(crate) use std::collections::hash_map::Entry;
    #[cfg(feature = "__sqlite-shared")]
    pub(crate) use std::panic::catch_unwind;
    #[cfg(feature = "__sqlite-shared")]
    pub(crate) use std::process::abort;
    #[cfg(feature = "__sqlite-shared")]
    pub(crate) use std::thread::panicking;
}
