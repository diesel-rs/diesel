/// Used in conjuction with
/// [`on_conflict`](trait.OnConflictExtension.html#method.on_conflict) to write
/// a query in the form `ON CONFLICT (name) DO NOTHING`. If you want to do
/// nothing when *any* constraint conflicts, use
/// [`on_conflict_do_nothing()`](trait.OnConflictExtension.html#method.on_conflict_do_nothing)
/// instead.
pub fn do_nothing() -> DoNothing {
    DoNothing
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct DoNothing;
