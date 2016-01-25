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

use expression::{AsExpression, SelectableExpression};
use expression::expression_methods::*;
use expression::helper_types::Eq;
use helper_types::FindBy;

/// Attempts to find a single record from the given table by primary key.
///
/// # Example
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
/// #     use diesel::result::Error::NotFound;
/// #     let connection = establish_connection();
/// let sean = (1, "Sean".to_string());
/// let tess = (2, "Tess".to_string());
/// assert_eq!(Ok(sean), users.find(1).first(&connection));
/// assert_eq!(Ok(tess), users.find(2).first(&connection));
/// assert_eq!(Err::<(i32, String), _>(NotFound), users.find(3).first(&connection));
/// # }
/// ```
pub trait FindDsl<PK> where
    Self: Table + FilterDsl<Eq<<Self as Table>::PrimaryKey, PK>>,
    PK: AsExpression<<Self::PrimaryKey as Expression>::SqlType>,
    Eq<Self::PrimaryKey, PK>: SelectableExpression<Self, SqlType=Bool> + NonAggregate,
{
    fn find(self, id: PK) -> FindBy<Self, Self::PrimaryKey, PK> {
        let primary_key = self.primary_key();
        self.filter(primary_key.eq(id))
    }
}

impl<T, PK> FindDsl<PK> for T where
    T: Table + FilterDsl<Eq<<T as Table>::PrimaryKey, PK>>,
    PK: AsExpression<<T::PrimaryKey as Expression>::SqlType>,
    Eq<T::PrimaryKey, PK>: SelectableExpression<T, SqlType=Bool> + NonAggregate,
{}
