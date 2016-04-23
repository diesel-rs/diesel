use query_builder::AsQuery;

pub trait BelongingToDsl<T: ?Sized> {
    type Output: AsQuery;

    fn belonging_to(other: &T) -> Self::Output;
}
