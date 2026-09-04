use crate::query_builder::insert_statement::{BatchInsert, InsertFromSelect};
use crate::query_builder::{BoxedSelectStatement, Query, SelectStatement, ValuesClause};

mod sealed {
    pub trait Sealed {}
}

/// Represents a type that can be converted into a value clause for an
/// `ON CONFLICT` statement.
///
/// This trait is sealed and cannot be implemented for types outside of Diesel,
/// and may be used to constrain generic parameters.
pub trait IntoConflictValueClause: sealed::Sealed {
    #[doc(hidden)]
    type ValueClause;

    #[doc(hidden)]
    fn into_value_clause(self) -> Self::ValueClause;
}

#[derive(Debug, Clone, Copy)]
pub struct OnConflictSelectWrapper<S>(pub(crate) S);

impl<Q> Query for OnConflictSelectWrapper<Q>
where
    Q: Query,
{
    type SqlType = Q::SqlType;
}

impl<Inner, Tab> sealed::Sealed for ValuesClause<Inner, Tab> {}
impl<Inner, Tab> IntoConflictValueClause for ValuesClause<Inner, Tab> {
    type ValueClause = Self;

    fn into_value_clause(self) -> Self::ValueClause {
        self
    }
}

impl<V, Tab, QId, const STATIC_QUERY_ID: bool> sealed::Sealed
    for BatchInsert<V, Tab, QId, STATIC_QUERY_ID>
{
}
impl<V, Tab, QId, const STATIC_QUERY_ID: bool> IntoConflictValueClause
    for BatchInsert<V, Tab, QId, STATIC_QUERY_ID>
{
    type ValueClause = Self;

    fn into_value_clause(self) -> Self::ValueClause {
        self
    }
}

impl<F, S, D, W, O, LOf, G, H, LC, Columns> sealed::Sealed
    for InsertFromSelect<SelectStatement<F, S, D, W, O, LOf, G, H, LC>, Columns>
{
}
impl<F, S, D, W, O, LOf, G, H, LC, Columns> IntoConflictValueClause
    for InsertFromSelect<SelectStatement<F, S, D, W, O, LOf, G, H, LC>, Columns>
{
    type ValueClause = InsertFromSelect<
        OnConflictSelectWrapper<SelectStatement<F, S, D, W, O, LOf, G, H, LC>>,
        Columns,
    >;

    fn into_value_clause(self) -> Self::ValueClause {
        let InsertFromSelect { columns, query } = self;
        InsertFromSelect {
            query: OnConflictSelectWrapper(query),
            columns,
        }
    }
}

impl<'a, ST, QS, DB, GB, Columns> sealed::Sealed
    for InsertFromSelect<BoxedSelectStatement<'a, ST, QS, DB, GB>, Columns>
{
}
impl<'a, ST, QS, DB, GB, Columns> IntoConflictValueClause
    for InsertFromSelect<BoxedSelectStatement<'a, ST, QS, DB, GB>, Columns>
{
    type ValueClause = InsertFromSelect<
        OnConflictSelectWrapper<BoxedSelectStatement<'a, ST, QS, DB, GB>>,
        Columns,
    >;

    fn into_value_clause(self) -> Self::ValueClause {
        let InsertFromSelect { columns, query } = self;
        InsertFromSelect {
            query: OnConflictSelectWrapper(query),
            columns,
        }
    }
}
