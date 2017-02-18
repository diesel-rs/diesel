#[macro_export]
macro_rules! hlist {
    // Empty list
    () => { $crate::hlist::Nil };

    // List without trailing commas
    ($($item:expr),+) => { hlist!($($item,)+) };

    // List with at least one item
    ($first:expr, $($rest:expr,)*) => {
        $crate::hlist::Cons($first, hlist!($($rest,)*))
    };
}

#[macro_export]
macro_rules! Hlist {
    // Empty list
    () => { $crate::hlist::Nil };

    // List without trailing commas
    ($($item:ty),+) => { Hlist!($($item,)+) };

    // List with at least one item
    ($first:ty, $($rest:ty,)*) => {
        $crate::hlist::Cons<$first, Hlist!($($rest,)*)>
    };
}

#[cfg(test)]
mod tests {
    use hlist::*;

    #[test]
    fn empty_hlist() {
        let hlist: Hlist!() = hlist!();
        assert_eq!(Nil, hlist);

        // We can't use assert_matches! here because there's only one pattern
        // that compiles
        match hlist {
            hlist!() => {} // this would fail to compile if the pattern were wrong
        }
    }

    #[test]
    fn one_item_hlist() {
        let hlist: Hlist!(i32) = hlist!(1);
        assert_eq!(Cons(1, Nil), hlist);
        assert_matches!(hlist, hlist!(1));

        let hlist: Hlist!(i32,) = hlist!(2,);
        assert_eq!(Cons(2, Nil), hlist);
        assert_matches!(hlist, hlist!(2,));

        let hlist: Hlist!(&str) = hlist!("hello");
        assert_eq!(Cons("hello", Nil), hlist);
        assert_matches!(hlist, hlist!("hello"));

        let hlist: Hlist!(&str,) = hlist!("world",);
        assert_eq!(Cons("world", Nil), hlist);
        assert_matches!(hlist, hlist!("world",));
    }

    #[test]
    fn multi_item_hlist() {
        let hlist: Hlist!(i32, i32) = hlist!(1, 2);
        assert_eq!(Cons(1, Cons(2, Nil)), hlist);
        assert_matches!(hlist, hlist!(1, 2));

        let hlist: Hlist!(i32, i32,) = hlist!(2, 3,);
        assert_eq!(Cons(2, Cons(3, Nil)), hlist);
        assert_matches!(hlist, hlist!(2, 3,));

        let str_hlist: Hlist!(&str, &str) = hlist!("hello", "world");
        assert_eq!(Cons("hello", Cons("world", Nil)), str_hlist);
        assert_matches!(str_hlist, hlist!("hello", "world"));

        let mixed_hlist: Hlist!(&str, i32, &str,) = hlist!("hello", 1, "world",);
        assert_eq!(Cons("hello", Cons(1, Cons("world", Nil))), mixed_hlist);
        assert_matches!(mixed_hlist, hlist!("hello", 1, "world",));
    }
}
