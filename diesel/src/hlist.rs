use std::fmt::{Debug, Formatter, Error as FmtError};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cons<Head, Tail>(pub Head, pub Tail);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nil;

pub trait Hlist {
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
