/// The `on_conflict` method
pub trait OnConflictDsl<Target> {
    /// The type returned by `.on_conflict`.
    type Output;

    /// See the trait documentation.
    fn on_conflict(self, target: Target) -> Self::Output;
}

/// The `on_conflict_do_nothing` method
pub trait OnConflictDoNothingDsl {
    /// The type returned by `.on_conflict_do_nothing`.
    type Output;

    /// See the trait documentation.
    fn on_conflict_do_nothing(self) -> Self::Output;
}
