use query_builder::AsQuery;

pub trait BelongingToDsl<T: ?Sized, FK> {
    type Output: AsQuery;

    fn belonging_to(other: &T) -> Self::Output;
}
