#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cons<Head, Tail>(pub Head, pub Tail);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
