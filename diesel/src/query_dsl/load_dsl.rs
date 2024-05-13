use self::private::LoadIter;
use super::RunQueryDsl;
use crate::backend::Backend;
use crate::connection::{Connection, DefaultLoadingMode, LoadConnection};
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::result::QueryResult;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::private::CompatibleType;

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
pub(crate) use self::private::CompatibleType;

/// The `load` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `load` from generic code.
///
/// [`RunQueryDsl`]: crate::RunQueryDsl
pub trait LoadQuery<'query, Conn, U, B = DefaultLoadingMode>: RunQueryDsl<Conn> {
    /// Return type of `LoadQuery::internal_load`
    type RowIter<'conn>: Iterator<Item = QueryResult<U>>
    where
        Conn: 'conn;

    /// Load this query
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn internal_load(self, conn: &mut Conn) -> QueryResult<Self::RowIter<'_>>;
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(note = "Use `LoadQuery::Iter` directly")]
pub type LoadRet<'conn, 'query, Q, C, U, B = DefaultLoadingMode> =
    <Q as LoadQuery<'query, C, U, B>>::RowIter<'conn>;

impl<'query, Conn, T, U, DB, B> LoadQuery<'query, Conn, U, B> for T
where
    Conn: Connection<Backend = DB> + LoadConnection<B>,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<DB> + QueryId + 'query,
    T::SqlType: CompatibleType<U, DB>,
    DB: Backend + QueryMetadata<T::SqlType> + 'static,
    U: FromSqlRow<<T::SqlType as CompatibleType<U, DB>>::SqlType, DB> + 'static,
    <T::SqlType as CompatibleType<U, DB>>::SqlType: 'static,
{
    type RowIter<'conn> = LoadIter<
        U,
        <Conn as LoadConnection<B>>::Cursor<'conn, 'query>,
        <T::SqlType as CompatibleType<U, DB>>::SqlType,
        DB,
    > where Conn: 'conn;

    fn internal_load(self, conn: &mut Conn) -> QueryResult<Self::RowIter<'_>> {
        Ok(LoadIter {
            cursor: conn.load(self.as_query())?,
            _marker: Default::default(),
        })
    }
}

/// The `execute` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `execute` from generic code.
///
/// [`RunQueryDsl`]: crate::RunQueryDsl
pub trait ExecuteDsl<Conn: Connection<Backend = DB>, DB: Backend = <Conn as Connection>::Backend>:
    Sized
{
    /// Execute this command
    fn execute(query: Self, conn: &mut Conn) -> QueryResult<usize>;
}

use crate::result::Error;

impl<Conn, DB, T> ExecuteDsl<Conn, DB> for T
where
    Conn: Connection<Backend = DB>,
    DB: Backend,
    T: QueryFragment<DB> + QueryId,
{
    fn execute(query: T, conn: &mut Conn) -> Result<usize, Error> {
        conn.execute_returning_count(&query)
    }
}

// These types and traits are not part of the public API.
//
// * CompatibleType as we consider this as "sealed" trait. It shouldn't
// be implemented by a third party
// * LoadIter as it's an implementation detail
mod private {
    use crate::backend::Backend;
    use crate::deserialize::FromSqlRow;
    use crate::expression::select_by::SelectBy;
    use crate::expression::{Expression, TypedExpressionType};
    use crate::sql_types::{SqlType, Untyped};
    use crate::{QueryResult, Selectable};

    #[allow(missing_debug_implementations)]
    pub struct LoadIter<U, C, ST, DB> {
        pub(super) cursor: C,
        pub(super) _marker: std::marker::PhantomData<(ST, U, DB)>,
    }

    impl<'a, C, U, ST, DB, R> LoadIter<U, C, ST, DB>
    where
        DB: Backend,
        C: Iterator<Item = QueryResult<R>>,
        R: crate::row::Row<'a, DB>,
        U: FromSqlRow<ST, DB>,
    {
        pub(super) fn map_row(row: Option<QueryResult<R>>) -> Option<QueryResult<U>> {
            match row? {
                Ok(row) => Some(
                    U::build_from_row(&row).map_err(crate::result::Error::DeserializationError),
                ),
                Err(e) => Some(Err(e)),
            }
        }
    }

    impl<'a, C, U, ST, DB, R> Iterator for LoadIter<U, C, ST, DB>
    where
        DB: Backend,
        C: Iterator<Item = QueryResult<R>>,
        R: crate::row::Row<'a, DB>,
        U: FromSqlRow<ST, DB>,
    {
        type Item = QueryResult<U>;

        fn next(&mut self) -> Option<Self::Item> {
            Self::map_row(self.cursor.next())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.cursor.size_hint()
        }

        fn count(self) -> usize
        where
            Self: Sized,
        {
            self.cursor.count()
        }

        fn last(self) -> Option<Self::Item>
        where
            Self: Sized,
        {
            Self::map_row(self.cursor.last())
        }

        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            Self::map_row(self.cursor.nth(n))
        }
    }

    impl<'a, C, U, ST, DB, R> ExactSizeIterator for LoadIter<U, C, ST, DB>
    where
        DB: Backend,
        C: ExactSizeIterator + Iterator<Item = QueryResult<R>>,
        R: crate::row::Row<'a, DB>,
        U: FromSqlRow<ST, DB>,
    {
        fn len(&self) -> usize {
            self.cursor.len()
        }
    }

    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    #[diagnostic::on_unimplemented(
        note = "this is a mismatch between what your query returns and what your type expects the query to return",
        note = "the fields in your struct need to match the fields returned by your query in count, order and type",
        note = "consider using `#[derive(Selectable)]` + `#[diesel(check_for_backend({DB}))]` on your struct `{U}` and \n\
                in your query `.select({U}::as_select())` to get a better error message"
    )]
    pub trait CompatibleType<U, DB> {
        type SqlType;
    }

    impl<ST, U, DB> CompatibleType<U, DB> for ST
    where
        DB: Backend,
        ST: SqlType + crate::sql_types::SingleValue,
        U: FromSqlRow<ST, DB>,
    {
        type SqlType = ST;
    }

    impl<U, DB> CompatibleType<U, DB> for Untyped
    where
        U: FromSqlRow<Untyped, DB>,
        DB: Backend,
    {
        type SqlType = Untyped;
    }

    impl<U, DB, E, ST> CompatibleType<U, DB> for SelectBy<U, DB>
    where
        DB: Backend,
        ST: SqlType + TypedExpressionType,
        U: Selectable<DB, SelectExpression = E>,
        E: Expression<SqlType = ST>,
        U: FromSqlRow<ST, DB>,
    {
        type SqlType = ST;
    }
}
