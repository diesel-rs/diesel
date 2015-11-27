use query_builder::AsQuery;

pub trait BelongingToDsl<T> {
    type Output: AsQuery;

    fn belonging_to(other: &T) -> Self::Output;
}
