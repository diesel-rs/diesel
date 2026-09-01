simple_clause!(
    /// A query node indicating that no group-by clause is set.
    NoGroupByClause,
    /// A query node containing a group-by clause.
    ///
    /// ```
    /// # #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    /// # fn main() {
    /// use diesel::query_builder::{GroupByClause, NoGroupByClause, SelectStatement};
    ///
    /// trait SelectionMarker {}
    ///
    /// impl<F, S, D, W, O, LOf, H, LC> SelectionMarker
    ///     for SelectStatement<F, S, D, W, O, LOf, NoGroupByClause, H, LC>
    /// {
    /// }
    ///
    /// impl<F, S, D, W, O, LOf, GB, H, LC> SelectionMarker
    ///     for SelectStatement<F, S, D, W, O, LOf, GroupByClause<GB>, H, LC>
    /// {
    /// }
    /// # }
    /// # #[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
    /// # fn main() {}
    /// ```
    GroupByClause,
    " GROUP BY "
);

pub trait ValidGroupByClause {
    type Expressions;
}

impl ValidGroupByClause for NoGroupByClause {
    type Expressions = ();
}

impl<GB> ValidGroupByClause for GroupByClause<GB> {
    type Expressions = GB;
}
