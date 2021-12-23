simple_clause!(
    /// A query node indicating the absence of a limit clause
    ///
    /// This type is only relevant for implementing custom backends
    NoLimitClause,
    /// A query node representing a limit clause
    ///
    /// This type is only relevant for implementing custom backends
    LimitClause,
    " LIMIT "
);
