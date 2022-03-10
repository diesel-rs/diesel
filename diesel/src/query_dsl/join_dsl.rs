use crate::helper_types;
use crate::query_builder::AsQuery;
use crate::query_source::joins::OnClauseWrapper;
use crate::query_source::{JoinTo, QuerySource, Table};

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors
pub trait InternalJoinDsl<Rhs, Kind, On> {
    type Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output;
}

impl<T, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for T
where
    T: Table + AsQuery,
    T::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <T::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors and grab
/// the known on clause from the associations API
pub trait JoinWithImplicitOnClause<Rhs, Kind> {
    type Output;

    fn join_with_implicit_on_clause(self, rhs: Rhs, kind: Kind) -> Self::Output;
}

impl<Lhs, Rhs, Kind> JoinWithImplicitOnClause<Rhs, Kind> for Lhs
where
    Lhs: JoinTo<Rhs>,
    Lhs: InternalJoinDsl<<Lhs as JoinTo<Rhs>>::FromClause, Kind, <Lhs as JoinTo<Rhs>>::OnClause>,
{
    type Output = <Lhs as InternalJoinDsl<Lhs::FromClause, Kind, Lhs::OnClause>>::Output;

    fn join_with_implicit_on_clause(self, rhs: Rhs, kind: Kind) -> Self::Output {
        let (from, on) = Lhs::join_target(rhs);
        self.join(from, kind, on)
    }
}

/// Specify the `ON` clause for a join statement. This will override
/// any implicit `ON` clause that would come from [`joinable!`]
///
/// [`joinable!`]: crate::joinable!
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::{users, posts};
/// #
/// # fn main() {
/// #     let connection = &mut establish_connection();
/// let data = users::table
///     .left_join(posts::table.on(
///         users::id.eq(posts::user_id).and(
///             posts::title.eq("My first post"))
///     ))
///     .select((users::name, posts::title.nullable()))
///     .load(connection);
/// let expected = vec![
///     ("Sean".to_string(), Some("My first post".to_string())),
///     ("Tess".to_string(), None),
/// ];
/// assert_eq!(Ok(expected), data);
/// # }
pub trait JoinOnDsl: Sized {
    /// See the trait documentation.
    fn on<On>(self, on: On) -> helper_types::On<Self, On> {
        OnClauseWrapper::new(self, on)
    }
}

impl<T: QuerySource> JoinOnDsl for T {}
