#[macro_export]
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
    macro_rules! hlist {
        ($($args:tt)*) => { diesel_hlist!($($args)*) }
    }

    #[macro_export]
    macro_rules! Hlist {
        ($($args:tt)*) => { DieselHlist!($($args)*) }
    }

    #[macro_export]
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
