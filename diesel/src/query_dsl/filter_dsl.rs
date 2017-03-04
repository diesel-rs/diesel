use expression::expression_methods::*;
use helper_types::Filter;
use query_builder::{AsQuery, Query};
use query_source::*;

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
/// #         id -> Integer,
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
pub trait FilterDsl<Predicate>: AsQuery {
    type Output: AsQuery<SqlType=Self::SqlType>;

    fn filter(self, predicate: Predicate) -> Self::Output;
}

impl<T, U, Predicate> FilterDsl<Predicate> for T where
    T: QuerySource + AsQuery<SqlType=<U as Query>::SqlType, Query=U>,
    U: Query + FilterDsl<Predicate, SqlType=<U as Query>::SqlType>,
{
    type Output = Filter<U, Predicate>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.as_query().filter(predicate)
    }
}

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
/// #         id -> Integer,
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
pub trait FindDsl<PK>: AsQuery {
    type Output: AsQuery<SqlType=Self::SqlType>;

    fn find(self, id: PK) -> Self::Output;
}

impl<T, PK> FindDsl<PK> for T where
    T: Table + FilterDsl<<<T as Table>::PrimaryKey as EqAll<PK>>::Output>,
    T::PrimaryKey: EqAll<PK>,
{
    type Output = Filter<Self, <T::PrimaryKey as EqAll<PK>>::Output>;

    fn find(self, id: PK) -> Self::Output {
        let primary_key = self.primary_key();
        self.filter(primary_key.eq_all(id))
    }
}
