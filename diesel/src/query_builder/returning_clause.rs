use crate::backend::SupportsReturningClause;

simple_clause_returning!(
    NoReturningClause,
    ReturningClause,
    " RETURNING ",
    backend_bounds = SupportsReturningClause
    (ReturningClauseWithSelect),
    " SELECT ",
    backend_bounds_with_select = SupportsReturningClause
);
