/// The `set` method
pub trait SetUpdateDsl<V> {
    /// The type returned by `.set`.
    type Output;

    /// See the trait documentation.
    fn set(self, values: V) -> Self::Output;
}
