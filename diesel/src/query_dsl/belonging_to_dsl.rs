pub trait BelongingToDsl<T> {
    type Output;

    fn belonging_to(other: T) -> Self::Output;
}
