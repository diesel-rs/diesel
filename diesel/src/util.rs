/// Treats tuples as a list which can be appended to. e.g.
/// `(a,).tuple_append(b) == (a, b)`
pub trait TupleAppend<T> {
    type Output;

    fn tuple_append(self, right: T) -> Self::Output;
}

pub trait TupleSize {
    const SIZE: usize;
}
