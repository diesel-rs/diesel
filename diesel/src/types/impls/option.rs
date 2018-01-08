use std::error::Error;
use std::fmt;
use std::io::Write;

use backend::Backend;
use expression::*;
use expression::bound::Bound;
use query_builder::QueryId;
use query_source::{Queryable, QueryableByName};
use row::NamedRow;
use types::{FromSql, FromSqlRow, HasSqlType, IsNull, NotNull, Nullable, ToSql, ToSqlOutput};
use {deserialize, serialize};

impl<T, DB> HasSqlType<Nullable<T>> for DB
where
    DB: Backend + HasSqlType<T>,
    T: NotNull,
{
    fn metadata(lookup: &DB::MetadataLookup) -> DB::TypeMetadata {
        <DB as HasSqlType<T>>::metadata(lookup)
    }

    fn row_metadata(out: &mut Vec<DB::TypeMetadata>, lookup: &DB::MetadataLookup) {
        <DB as HasSqlType<T>>::row_metadata(out, lookup)
    }
}

impl<T> QueryId for Nullable<T>
where
    T: QueryId + NotNull,
{
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, ST, DB> FromSql<Nullable<ST>, DB> for Option<T>
where
    T: FromSql<ST, DB>,
    DB: Backend,
    ST: NotNull,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match bytes {
            Some(_) => T::from_sql(bytes).map(Some),
            None => Ok(None),
        }
    }
}

impl<T, ST, DB> Queryable<Nullable<ST>, DB> for Option<T>
where
    T: Queryable<ST, DB>,
    DB: Backend,
    Option<T::Row>: FromSqlRow<Nullable<ST>, DB>,
    ST: NotNull,
{
    type Row = Option<T::Row>;

    fn build(row: Self::Row) -> Self {
        row.map(T::build)
    }
}

impl<T, DB> QueryableByName<DB> for Option<T>
where
    T: QueryableByName<DB>,
    DB: Backend,
{
    fn build<R: NamedRow<DB>>(row: &R) -> deserialize::Result<Self> {
        match T::build(row) {
            Ok(v) => Ok(Some(v)),
            Err(e) => if e.is::<UnexpectedNullError>() {
                Ok(None)
            } else {
                Err(e)
            },
        }
    }
}

impl<T, ST, DB> FromSqlRow<Nullable<ST>, DB> for Option<T>
where
    T: FromSqlRow<ST, DB>,
    DB: Backend,
    ST: NotNull,
{
    const FIELDS_NEEDED: usize = T::FIELDS_NEEDED;

    fn build_from_row<R: ::row::Row<DB>>(row: &mut R) -> deserialize::Result<Self> {
        let fields_needed = Self::FIELDS_NEEDED;
        if row.next_is_null(fields_needed) {
            row.advance(fields_needed);
            Ok(None)
        } else {
            T::build_from_row(row).map(Some)
        }
    }
}

impl<T, ST, DB> ToSql<Nullable<ST>, DB> for Option<T>
where
    T: ToSql<ST, DB>,
    DB: Backend,
    ST: NotNull,
{
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, DB>) -> serialize::Result {
        if let Some(ref value) = *self {
            value.to_sql(out)
        } else {
            Ok(IsNull::Yes)
        }
    }
}

impl<T, ST> AsExpression<Nullable<ST>> for Option<T>
where
    ST: NotNull,
{
    type Expression = Bound<Nullable<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'a, T, ST> AsExpression<Nullable<ST>> for &'a Option<T>
where
    ST: NotNull,
{
    type Expression = Bound<Nullable<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

#[derive(Debug)]
pub struct UnexpectedNullError {
    pub msg: String,
}

impl fmt::Display for UnexpectedNullError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for UnexpectedNullError {
    fn description(&self) -> &str {
        &self.msg
    }
}

#[cfg(all(test, feature = "postgres"))]
use types;
#[cfg(all(test, feature = "postgres"))]
use pg::Pg;

#[test]
#[cfg(feature = "postgres")]
fn option_to_sql() {
    type Type = types::Nullable<types::VarChar>;
    let mut bytes = ToSqlOutput::test();

    let is_null = ToSql::<Type, Pg>::to_sql(&None::<String>, &mut bytes).unwrap();
    assert_eq!(IsNull::Yes, is_null);
    assert!(bytes.is_empty());

    let is_null = ToSql::<Type, Pg>::to_sql(&Some(""), &mut bytes).unwrap();
    assert_eq!(IsNull::No, is_null);
    assert!(bytes.is_empty());

    let is_null = ToSql::<Type, Pg>::to_sql(&Some("Sean"), &mut bytes).unwrap();
    let expectd_bytes = b"Sean".to_vec();
    assert_eq!(IsNull::No, is_null);
    assert_eq!(bytes, expectd_bytes);
}
