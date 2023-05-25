use crate::backend::Backend;
use crate::deserialize::{self, FromSql, Queryable, QueryableByName};
use crate::expression::bound::Bound;
use crate::expression::*;
use crate::query_builder::QueryId;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{is_nullable, HasSqlType, Nullable, SingleValue, SqlType};
use crate::NullableExpressionMethods;

impl<T, DB> HasSqlType<Nullable<T>> for DB
where
    DB: Backend + HasSqlType<T>,
    T: SqlType,
{
    fn metadata(lookup: &mut DB::MetadataLookup) -> DB::TypeMetadata {
        <DB as HasSqlType<T>>::metadata(lookup)
    }
}

impl<T> QueryId for Nullable<T>
where
    T: QueryId + SqlType<IsNull = is_nullable::NotNull>,
{
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, ST, DB> FromSql<Nullable<ST>, DB> for Option<T>
where
    T: FromSql<ST, DB>,
    DB: Backend,
    ST: SqlType<IsNull = is_nullable::NotNull>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        T::from_sql(bytes).map(Some)
    }

    fn from_nullable_sql(bytes: Option<DB::RawValue<'_>>) -> deserialize::Result<Self> {
        match bytes {
            Some(bytes) => T::from_sql(bytes).map(Some),
            None => Ok(None),
        }
    }
}

impl<T, ST, DB> ToSql<Nullable<ST>, DB> for Option<T>
where
    T: ToSql<ST, DB>,
    DB: Backend,
    ST: SqlType<IsNull = is_nullable::NotNull>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        if let Some(ref value) = *self {
            value.to_sql(out)
        } else {
            Ok(IsNull::Yes)
        }
    }
}

impl<T, ST> AsExpression<Nullable<ST>> for Option<T>
where
    ST: SqlType<IsNull = is_nullable::NotNull>,
    Nullable<ST>: TypedExpressionType,
{
    type Expression = Bound<Nullable<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'a, T, ST> AsExpression<Nullable<ST>> for &'a Option<T>
where
    ST: SqlType<IsNull = is_nullable::NotNull>,
    Nullable<ST>: TypedExpressionType,
{
    type Expression = Bound<Nullable<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<T, DB> QueryableByName<DB> for Option<T>
where
    DB: Backend,
    T: QueryableByName<DB>,
{
    fn build<'a>(row: &impl crate::row::NamedRow<'a, DB>) -> deserialize::Result<Self> {
        match T::build(row) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.is::<crate::result::UnexpectedNullError>() => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<ST, T, DB> Queryable<ST, DB> for Option<T>
where
    ST: SingleValue<IsNull = is_nullable::IsNullable>,
    DB: Backend,
    Self: FromSql<ST, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<T, DB> Selectable<DB> for Option<T>
where
    DB: Backend,
    T: Selectable<DB>,
    crate::dsl::Nullable<T::SelectExpression>: Expression,
{
    type SelectExpression = crate::dsl::Nullable<T::SelectExpression>;

    fn construct_selection() -> Self::SelectExpression {
        T::construct_selection().nullable()
    }
}

#[test]
#[cfg(feature = "postgres")]
fn option_to_sql() {
    use crate::pg::Pg;
    use crate::query_builder::bind_collector::ByteWrapper;
    use crate::sql_types;

    type Type = sql_types::Nullable<sql_types::VarChar>;

    let mut buffer = Vec::new();
    let is_null = {
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<Type, Pg>::to_sql(&None::<String>, &mut bytes).unwrap()
    };
    assert_eq!(IsNull::Yes, is_null);
    assert!(buffer.is_empty());

    let is_null = {
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<Type, Pg>::to_sql(&Some(""), &mut bytes).unwrap()
    };
    assert_eq!(IsNull::No, is_null);
    assert!(buffer.is_empty());

    let is_null = {
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<Type, Pg>::to_sql(&Some("Sean"), &mut bytes).unwrap()
    };
    let expected_bytes = b"Sean".to_vec();
    assert_eq!(IsNull::No, is_null);
    assert_eq!(buffer, expected_bytes);
}
