simple_clause!(
    /// A query node indicating the absence of an offset clause
    ///
    /// This type is only relevant for implementing custom backends
    NoOffsetClause,
    /// A query node representing an offset clause
    ///
    /// This type is only relevant for implementing custom backends
    OffsetClause,
    " OFFSET "
);
