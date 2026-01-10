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

/// The `do_nothing` method
pub trait DoNothingDsl {
    /// The type returned by `.do_nothing`.
    type Output;

    /// See the trait documentation.
    fn do_nothing(self) -> Self::Output;
}

/// The `do_update` method
pub trait DoUpdateDsl {
    /// The type returned by `.do_update`.
    type Output;

    /// See the trait documentation.
    fn do_update(self) -> Self::Output;
}
