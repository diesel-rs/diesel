use crate::backend::SupportsReturningClause;

simple_clause!(
    NoReturningClause,
    ReturningClause,
    " RETURNING ",
    backend_bounds = SupportsReturningClause
);
