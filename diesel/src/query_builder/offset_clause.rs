simple_clause!(
    /// A query node indicating the absence of an offset clause
    ///
    /// This type is only relevant for implementing custom backends
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
    )]
    NoOffsetClause,
    /// A query node representing an offset clause
    ///
    /// This type is only relevant for implementing custom backends
    #[cfg_attr(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
    )]
    OffsetClause,
    " OFFSET "
);
