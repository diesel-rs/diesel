pub trait TupleAppend<T> {
    type Output;

    fn tuple_append(self, right: T) -> Self::Output;
}
