use super::group_by_clause::ValidGroupByClause;
use super::{BoxedSelectStatement, FromClause, NoFromClause, Query, QueryId, SelectStatement};
use crate::backend::Backend;
use crate::expression::ValidGrouping;
use crate::query_dsl::methods::SelectDsl;
use crate::{AppearsOnTable, QuerySource, SelectableHelper};
pub use diesel_derives::HasQuery;

/// Trait indicating that a base query can be constructed for this type
///
/// Types which implement `HasQuery` have a default query for loading
/// the relevant data from the database associated with this type.
///
/// Consumers of this trait should use the `query()` associated function
/// to construct a query including a matching select clause for their type
///
/// This trait can be [derived](derive@HasQuery)
///
/// It's important to note that for Diesel mappings between the database and rust types always happen
/// on query and not on table level. This enables you to write several queries related to the
/// same table, while a single query could be related to zero or multiple tables.
///
/// # Example
///
/// ## With derive
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenvy;
/// # include!("../doctest_setup.rs");
/// #
///
/// // it's important to have the right table in scope
/// use schema::users;
///
/// #[derive(HasQuery, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() -> QueryResult<()> {
/// #
/// #     let connection = &mut establish_connection();
/// // equivalent to `users::table.select(User::as_select()).first(connection)?;
/// let first_user = User::query().first(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
///
/// #     Ok(())
/// # }
/// ```
///
/// ## Manual implementation
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenvy;
/// # include!("../doctest_setup.rs");
/// #
///
/// // it's important to have the right table in scope
/// use schema::users;
///
/// #[derive(Selectable, Queryable, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// impl<DB: diesel::backend::Backend> diesel::HasQuery<DB> for User {
///    type BaseQuery = <users::table as diesel::query_builder::AsQuery>::Query;
///
///    // internal not stable method
///    fn base_query() -> Self::BaseQuery {
///        use diesel::query_builder::AsQuery;
///        users::table.as_query()
///    }
/// }
///
/// # fn main() -> QueryResult<()> {
/// #
/// #     let connection = &mut establish_connection();
/// // equivalent to `users::table.select(User::as_select()).first(connection)?;
/// let first_user = User::query().first(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
///
/// #     Ok(())
/// # }
/// ```
// Ideally we would have a `Queryable<Self::SelectExpression::SqlType, DB> as well here
// but rustc breaks down if we do that
// It claimns in that case that it isn't implemented,
// while it is obviously implemented by the derive
pub trait HasQuery<DB: Backend>:
    SelectableHelper<
    DB,
    SelectExpression: QueryId
            // these two bounds are here to get better error messages
                          + AppearsOnTable<<Self::BaseQuery as AcceptedQueries>::From>
                          + ValidGrouping<<Self::BaseQuery as AcceptedQueries>::GroupBy>,
>
{
    /// Base query type defined by the implementing type
    type BaseQuery: AcceptedQueries + SelectDsl<crate::dsl::AsSelect<Self, DB>, Output: Query>;

    #[doc(hidden)] // that method is for internal use only
    fn base_query() -> Self::BaseQuery;

    /// Construct the query associated with this type
    fn query() -> crate::dsl::Select<Self::BaseQuery, crate::dsl::AsSelect<Self, DB>> {
        Self::base_query().select(Self::as_select())
    }
}

use self::private::AcceptedQueries;

mod private {
    use super::*;
    pub trait AcceptedQueries {
        type From;
        type GroupBy;
    }

    impl<S, D, W, O, LOf, GB, H, L> AcceptedQueries
        for SelectStatement<NoFromClause, S, D, W, O, LOf, GB, H, L>
    where
        GB: ValidGroupByClause,
    {
        type From = NoFromClause;

        type GroupBy = GB::Expressions;
    }

    impl<F, S, D, W, O, LOf, GB, H, L> AcceptedQueries
        for SelectStatement<FromClause<F>, S, D, W, O, LOf, GB, H, L>
    where
        F: QuerySource,
        GB: ValidGroupByClause,
    {
        type From = F;

        type GroupBy = GB::Expressions;
    }

    impl<'a, ST, DB, GB> AcceptedQueries for BoxedSelectStatement<'a, ST, NoFromClause, DB, GB>
    where
        GB: ValidGroupByClause,
    {
        type From = NoFromClause;

        type GroupBy = GB::Expressions;
    }

    impl<'a, ST, F, DB, GB> AcceptedQueries for BoxedSelectStatement<'a, ST, FromClause<F>, DB, GB>
    where
        F: QuerySource,
        GB: ValidGroupByClause,
    {
        type From = F;

        type GroupBy = GB::Expressions;
    }
}
