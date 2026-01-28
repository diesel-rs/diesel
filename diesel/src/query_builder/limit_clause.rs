simple_clause!(
    /// A query node indicating the absence of a limit clause
    ///
    /// This type is only relevant for implementing custom backends
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
    )]
    NoLimitClause,
    /// A query node representing a limit clause
    ///
    /// This type is only relevant for implementing custom backends
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
    )]
    LimitClause,
    " LIMIT "
);
