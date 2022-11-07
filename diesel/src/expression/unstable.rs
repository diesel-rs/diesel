use crate::expression::{is_aggregate, MixedAggregates, ValidGrouping};

// Note that these docs are similar to but slightly different than the stable
// docs below. Make sure if you change these that you also change the docs
// below.
/// Trait alias to represent an expression that isn't aggregate by default.
///
/// This alias represents a type which is not aggregate if there is no group by
/// clause. More specifically, it represents for types which implement
/// [`ValidGrouping<()>`] where `IsAggregate` is [`is_aggregate::No`] or
/// [`is_aggregate::Yes`].
///
/// While this trait is a useful stand-in for common cases, `T: NonAggregate`
/// cannot always be used when `T: ValidGrouping<(), IsAggregate = No>` or
/// `T: ValidGrouping<(), IsAggregate = Never>` could be. For that reason,
/// unless you need to abstract over both columns and literals, you should
/// prefer to use [`ValidGrouping<()>`] in your bounds instead.
///
/// [`ValidGrouping<()>`]: ValidGrouping
pub trait NonAggregate = ValidGrouping<()>
where
    <Self as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>;
