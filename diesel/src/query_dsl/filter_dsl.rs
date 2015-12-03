use expression::{Expression, NonAggregate};
use query_builder::AsQuery;
use query_source::filter::FilteredQuerySource;
use query_source::{Table, InnerJoinSource, LeftOuterJoinSource};
use types::Bool;

/// Adds to the `WHERE` clause of a query. If there is already a `WHERE` clause,
/// the result will be `old AND new`. This is automatically implemented for the
/// various query builder types.
///
/// # Example:
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// let seans_id = users.filter(name.eq("Sean")).select(id)
///     .first(&connection);
/// assert_eq!(Ok(1), seans_id);
/// let tess_id = users.filter(name.eq("Tess")).select(id)
///     .first(&connection);
/// assert_eq!(Ok(2), tess_id);
/// # }
/// ```
pub trait FilterDsl<Predicate: Expression<SqlType=Bool> + NonAggregate> {
    type Output: AsQuery;

    fn filter(self, predicate: Predicate) -> Self::Output;
}

pub trait NotFiltered {
}

impl<T, Predicate> FilterDsl<Predicate> for T where
    Predicate: Expression<SqlType=Bool> + NonAggregate,
    FilteredQuerySource<T, Predicate>: AsQuery,
    T: NotFiltered,
{
    type Output = FilteredQuerySource<Self, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        FilteredQuerySource::new(self, predicate)
    }
}

impl<T: Table> NotFiltered for T {}
impl<Left, Right> NotFiltered for InnerJoinSource<Left, Right> {}
impl<Left, Right> NotFiltered for LeftOuterJoinSource<Left, Right> {}
