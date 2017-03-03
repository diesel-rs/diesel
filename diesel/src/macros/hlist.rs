#[macro_export]
/// Alias for `hlist!`.
///
/// Since `hlist!` is quite a generic name, we also export it with a more
/// specific name that is unlikely to collide with other crates.  The generic
/// names can be disabled by enabling the `conservative-macro-names` feature.
///
/// If you are writing a crate which adds features to diesel, not an
/// application, you should always use this macro instead of `hlist!` in order
/// to be compatible with applications which disable the more generic names.
macro_rules! diesel_hlist {
    // Empty list
    () => { $crate::hlist::Nil };

    // List without trailing commas
    ($($item:expr),+) => { diesel_hlist!($($item,)+) };

    // List with at least one item
    ($first:expr, $($rest:expr,)*) => {
        $crate::hlist::Cons($first, diesel_hlist!($($rest,)*))
    };
}

#[macro_export]
/// Alias for `Hlist!`.
///
/// Since `Hlist!` is quite a generic name, we also export it with a more
/// specific name that is unlikely to collide with other crates.  The generic
/// names can be disabled by enabling the `conservative-macro-names` feature.
///
/// If you are writing a crate which adds features to diesel, not an
/// application, you should always use this macro instead of `Hlist!` in order
/// to be compatible with applications which disable the more generic names.
macro_rules! DieselHlist {
    // Empty list
    () => { $crate::hlist::Nil };

    // List without trailing commas
    ($($item:ty),+) => { DieselHlist!($($item,)+) };

    // List with at least one item
    ($first:ty, $($rest:ty,)*) => {
        $crate::hlist::Cons<$first, DieselHlist!($($rest,)*)>
    };
}

#[macro_export]
/// Alias for `hlist_pat!`.
///
/// Since `hlist_pat!` is quite a generic name, we also export it with a more
/// specific name that is unlikely to collide with other crates.  The generic
/// names can be disabled by enabling the `conservative-macro-names` feature.
///
/// If you are writing a crate which adds features to diesel, not an
/// application, you should always use this macro instead of `hlist_pat!` in order
/// to be compatible with applications which disable the more generic names.
macro_rules! diesel_hlist_pat {
    // Empty list
    () => { $crate::hlist::Nil };

    // List without trailing commas
    ($($item:pat),+) => { diesel_hlist_pat!($($item,)+) };

    // List with at least one item
    ($first:pat, $($rest:pat,)*) => {
        $crate::hlist::Cons($first, diesel_hlist_pat!($($rest,)*))
    };
}

#[cfg(not(feature="conservative-macro-names"))]
#[macro_use]
mod hlist_rename {
    #[macro_export]
    /// Constructs a variable length argument list.
    ///
    /// A list of arguments to be passed to a function which takes a variable
    /// number of arguments, such as
    /// [`select`](prelude/trait.SelectDsl.html#tymethod.select) and
    /// [`order`](prelude/trait.OrderDsl.html#tymethod.order).
    ///
    /// Note: This macro has a generic name which may conflict with other
    /// crates. You can disable the generic names by enabling the
    /// `conservative-macro-names` feature. This macro is also exported as
    /// `diesel_hlist!`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// users.select(hlist!(id, name))
    /// ```
    macro_rules! hlist {
        ($($args:tt)*) => { diesel_hlist!($($args)*) }
    }

    #[macro_export]
    /// The type of a variable length argument list.
    ///
    /// Similar to `hlist!`, but used in type positions. The type of `hlist!(1,
    /// "foo", MyStruct)` will be `Hlist!(i32, &str, MyStruct)`.
    ///
    /// Note: This macro has a generic name which may conflict with other
    /// crates. You can disable the generic names by enabling the
    /// `conservative-macro-names` feature. This macro is also exported as
    /// `DieselHlist!`.
    macro_rules! Hlist {
        ($($args:tt)*) => { DieselHlist!($($args)*) }
    }

    #[macro_export]
    /// A pattern which matches variable length argument lists.
    ///
    /// Similar to `hlist!`, but used in pattern positions. A value of
    /// `hlist!(1, "foo", MyStruct)` will match `hlist_pat!(x, y, z)`.
    ///
    /// Note: This macro has a generic name which may conflict with other
    /// crates. You can disable the generic names by enabling the
    /// `conservative-macro-names` feature. This macro is also exported as
    /// `diesel_hlist_pat!`.
    macro_rules! hlist_pat {
        ($($args:tt)*) => { diesel_hlist_pat!($($args)*) }
    }
}

#[cfg(test)]
mod tests {
    use hlist::*;

    #[test]
    fn empty_hlist() {
        let hlist: DieselHlist!() = diesel_hlist!();
        assert_eq!(Nil, hlist);

        // We can't use assert_matches! here because there's only one pattern
        // that compiles
        match hlist {
            diesel_hlist!() => {} // this would fail to compile if the pattern were wrong
        }
    }

    #[test]
    fn one_item_hlist() {
        let hlist: DieselHlist!(i32) = diesel_hlist!(1);
        assert_eq!(Cons(1, Nil), hlist);
        assert_matches!(hlist, diesel_hlist!(1));

        let hlist: DieselHlist!(i32,) = diesel_hlist!(2,);
        assert_eq!(Cons(2, Nil), hlist);
        assert_matches!(hlist, diesel_hlist!(2,));

        let hlist: DieselHlist!(&str) = diesel_hlist!("hello");
        assert_eq!(Cons("hello", Nil), hlist);
        assert_matches!(hlist, diesel_hlist!("hello"));

        let hlist: DieselHlist!(&str,) = diesel_hlist!("world",);
        assert_eq!(Cons("world", Nil), hlist);
        assert_matches!(hlist, diesel_hlist!("world",));
    }

    #[test]
    fn multi_item_hlist() {
        let hlist: DieselHlist!(i32, i32) = diesel_hlist!(1, 2);
        assert_eq!(Cons(1, Cons(2, Nil)), hlist);
        assert_matches!(hlist, diesel_hlist!(1, 2));

        let hlist: DieselHlist!(i32, i32,) = diesel_hlist!(2, 3,);
        assert_eq!(Cons(2, Cons(3, Nil)), hlist);
        assert_matches!(hlist, diesel_hlist!(2, 3,));

        let str_hlist: DieselHlist!(&str, &str) = diesel_hlist!("hello", "world");
        assert_eq!(Cons("hello", Cons("world", Nil)), str_hlist);
        assert_matches!(str_hlist, diesel_hlist!("hello", "world"));

        let mixed_hlist: DieselHlist!(&str, i32, &str,) = diesel_hlist!("hello", 1, "world",);
        assert_eq!(Cons("hello", Cons(1, Cons("world", Nil))), mixed_hlist);
        assert_matches!(mixed_hlist, diesel_hlist!("hello", 1, "world",));
    }
}
