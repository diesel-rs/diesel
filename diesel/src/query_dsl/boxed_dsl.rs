use backend::Backend;
use query_builder::AsQuery;
use query_source::Table;

pub trait InternalBoxedDsl<'a, DB: Backend> {
    type Output;

    fn internal_into_boxed(self) -> Self::Output;
}

impl<'a, T, DB> InternalBoxedDsl<'a, DB> for T where
    DB: Backend,
    T: Table + AsQuery,
    T::Query: InternalBoxedDsl<'a, DB>,
{
    type Output = <T::Query as InternalBoxedDsl<'a, DB>>::Output;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}

/// Boxes the pieces of a query into a single type. This is useful for cases
/// where you want to conditionally modify a query, but need the type to remain
/// the same. The backend must be specified as part of this. It is not possible
/// to box a query and have it be useable on multiple backends.
///
/// A boxed query will incur a minor performance penalty, as the query builder
/// can no longer be inlined by the compiler. For most applications this cost
/// will be minimal.
///
/// ### Example
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
/// #     use std::collections::HashMap;
/// #     let connection = establish_connection();
/// #     let mut params = HashMap::new();
/// #     params.insert("name", "Sean");
/// let mut query = users::table.into_boxed();
/// if let Some(name) = params.get("name") {
///     query = query.filter(users::name.eq(name));
/// }
/// let users = query.load(&connection);
/// #     let expected = vec![(1, String::from("Sean"))];
/// #     assert_eq!(Ok(expected), users);
/// # }
/// ```
///
/// Diesel queries also have a similar problem to [`Iterator`][iterator], where
/// returning them from a function requires exposing the implementation of that
/// function. The [`helper_types`][helper_types] module exists to help with this,
/// but you might want to hide the return type or have it conditionally change.
/// Boxing can achieve both.
///
/// [iterator]: https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html
/// [helper_types]: ../helper_types/index.html
///
/// ### Example
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
/// #     let connection = establish_connection();
/// fn users_by_name<'a>(name: &'a str) -> users::BoxedQuery<'a, DB> {
///     users::table.filter(users::name.eq(name)).into_boxed()
/// }
///
/// assert_eq!(Ok(1), users_by_name("Sean").select(users::id).first(&connection));
/// assert_eq!(Ok(2), users_by_name("Tess").select(users::id).first(&connection));
/// # }
/// ```
pub trait BoxedDsl: Sized {
    fn into_boxed<'a, DB>(self) -> Self::Output where
        DB: Backend,
        Self: InternalBoxedDsl<'a, DB>,
    {
        self.internal_into_boxed()
    }
}

impl<T: AsQuery> BoxedDsl for T {}
