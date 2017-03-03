//! A heterogeneous list used by Diesel to emulate variadic functions.
//!
//! Most applications will not need to concern themselves with this module. See
//! the [`hlist!` macro](../macro.hlist.html) for common usage.
//!
//! A heterogeneous list, or hlist for short, is a singly linked list where the
//! type of each element may differ. This is represented statically in the type.
//!
//! Hlists are very similar to tuples in that they are finite, and heterogenous.
//! In general, you can think of `diesel_hlist!(1, "foo", MyStruct)` as
//! analogous to `(1, ("foo", (MyStruct, ())))`.
//!
//! However, they are significantly easier to work with in a generic context
//! than tuples. To implement a trait for all hlists, you need to implement that
//! trait for `Cons<Head, Tail>` and `Nil`. To implement a trait for all tuples,
//! you will need one impl per tuple size that you wish to support. The tradeoff
//! for this is that they are somewhat more difficult to work with in a
//! non-generic context. They cannot be indexed, they must be accessed via
//! pattern matching.
//!
//! Diesel exposes three macros to work with hlists. Hlists can be constructed
//! using [`diesel_hlist!`](../macro.diesel_hlist.html). The type of an hlist
//! can be referenced using [`DieselHlist!`](../macro.DieselHlist.html).
//! Patterns which match hlists can be constructed using
//! [`diesel_hlist_pat!`](../macro.diesel_hlist_pat.html). The type of
//! `diesel_hlist!(1, "foo", MyStruct)` is `DieselHlist!(i32, &str, MyStruct)`
//! and matches the pattern `diesel_hlist_pat!(x, y, z)`.

use std::fmt::{Debug, Formatter, Error as FmtError};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// An hlist which contains one or more items. `Tail` will always either be
/// `Cons` or `Nil`.
pub struct Cons<Head, Tail>(pub Head, pub Tail);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// An empty hlist.
pub struct Nil;

/// Utility trait for working with hlists
pub trait Hlist {
    /// The total length of a list.
    ///
    /// Since this is represented in the type, this function does not take
    /// `&self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # use diesel::hlist::*;
    /// # fn main() {
    /// assert_eq!(0, <DieselHlist!()>::len());
    /// assert_eq!(1, <DieselHlist!(i32)>::len());
    /// assert_eq!(2, <DieselHlist!(i32, &str)>::len());
    /// # }
    fn len() -> usize;
}

impl<T, U> Hlist for Cons<T, U> where
    U: Hlist,
{
    fn len() -> usize {
        U::len() + 1
    }
}

impl Hlist for Nil {
    fn len() -> usize {
        0
    }
}

impl Debug for Nil {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FmtError> {
        formatter.write_str("hlist()")
    }
}

impl<T, U> Debug for Cons<T, U> where
    Cons<T, U>: DebugItems,
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FmtError> {
        let mut items: &DebugItems = &*self;
        let mut formatter = formatter.debug_tuple("hlist");
        while let Some((head, tail)) = items.pop() {
            items = tail;
            formatter.field(head);
        }
        formatter.finish()
    }
}

#[doc(hidden)]
pub trait DebugItems {
    fn pop(&self) -> Option<(&Debug, &DebugItems)>;
}

impl<T, U> DebugItems for Cons<T, U> where
    T: Debug,
    U: DebugItems,
{
    fn pop(&self) -> Option<(&Debug, &DebugItems)> {
        Some((&self.0, &self.1))
    }
}

impl DebugItems for Nil {
    fn pop(&self) -> Option<(&Debug, &DebugItems)> {
        None
    }
}

#[test]
fn debug_empty_hlist() {
    assert_eq!("hlist()", format!("{:?}", Nil));
    assert_eq!("hlist()", format!("{:#?}", Nil));
}

#[test]
fn debug_single_item_hlist() {
    assert_eq!("hlist(1)", format!("{:?}", Cons(1, Nil)));
    assert_eq!("hlist(2)", format!("{:?}", Cons(2, Nil)));
    assert_eq!(r#"hlist("hello")"#, format!("{:?}", Cons("hello", Nil)));
    assert_eq!(r#"hlist("world")"#, format!("{:?}", Cons("world", Nil)));
}

#[test]
fn debug_multi_item_hlist() {
    assert_eq!("hlist(1, 2)", format!("{:?}", Cons(1, Cons(2, Nil))));
    assert_eq!("hlist(2, 3)", format!("{:?}", Cons(2, Cons(3, Nil))));
    let str_hlist = Cons("hello", Cons("world", Nil));
    assert_eq!(r#"hlist("hello", "world")"#, format!("{:?}", str_hlist));
    let mixed_hlist = Cons("hello", Cons(1, Cons("world", Cons(2, Nil))));
    assert_eq!(r#"hlist("hello", 1, "world", 2)"#, format!("{:?}", mixed_hlist));
}
