use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Oid, Pg> for u32 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = bytes.as_bytes();
        bytes.read_u32::<NetworkEndian>().map_err(Into::into)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Oid, Pg> for u32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_u32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::SmallInt, Pg> for i16 {
    #[inline(always)]
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        if bytes.len() < 2 {
            return emit_size_error(
                "Received less than 2 bytes while decoding an i16. \
                    Was an expression of a different type accidentally marked as SmallInt?",
            );
        }

        if bytes.len() > 2 {
            return emit_size_error(
                "Received more than 2 bytes while decoding an i16. \
                    Was an Integer expression accidentally marked as SmallInt?",
            );
        }
        bytes
            .read_i16::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Integer, Pg> for i32 {
    #[inline(always)]
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        if bytes.len() < 4 {
            return emit_size_error(
                "Received less than 4 bytes while decoding an i32. \
                    Was an SmallInt expression accidentally marked as Integer?",
            );
        }

        if bytes.len() > 4 {
            return emit_size_error(
                "Received more than 4 bytes while decoding an i32. \
                    Was an BigInt expression accidentally marked as Integer?",
            );
        }
        bytes
            .read_i32::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cold]
#[inline(never)]
fn emit_size_error<T>(var_name: &str) -> deserialize::Result<T> {
    deserialize::Result::Err(var_name.into())
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::BigInt, Pg> for i64 {
    #[inline(always)]
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        if bytes.len() < 8 {
            return emit_size_error(
                "Received less than 8 bytes while decoding an i64. \
                    Was an Integer expression accidentally marked as BigInt?",
            );
        }

        if bytes.len() > 8 {
            return emit_size_error(
                "Received more than 8 bytes while decoding an i64. \
                    Was an expression of a different type expression accidentally marked as BigInt?"
            );
        }
        bytes
            .read_i64::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::SmallInt, Pg> for i16 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_i16::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Integer, Pg> for i32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_i32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::BigInt, Pg> for i64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_i64::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_builder::bind_collector::ByteWrapper;

    #[test]
    fn i16_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<sql_types::SmallInt, Pg>::to_sql(&1i16, &mut bytes).unwrap();
        ToSql::<sql_types::SmallInt, Pg>::to_sql(&0i16, &mut bytes).unwrap();
        ToSql::<sql_types::SmallInt, Pg>::to_sql(&-1i16, &mut bytes).unwrap();
        assert_eq!(buffer, vec![0, 1, 0, 0, 255, 255]);
    }

    #[test]
    fn i32_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<sql_types::Integer, Pg>::to_sql(&1i32, &mut bytes).unwrap();
        ToSql::<sql_types::Integer, Pg>::to_sql(&0i32, &mut bytes).unwrap();
        ToSql::<sql_types::Integer, Pg>::to_sql(&-1i32, &mut bytes).unwrap();
        assert_eq!(buffer, vec![0, 0, 0, 1, 0, 0, 0, 0, 255, 255, 255, 255]);
    }

    #[test]
    fn i64_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<sql_types::BigInt, Pg>::to_sql(&1i64, &mut bytes).unwrap();
        ToSql::<sql_types::BigInt, Pg>::to_sql(&0i64, &mut bytes).unwrap();
        ToSql::<sql_types::BigInt, Pg>::to_sql(&-1i64, &mut bytes).unwrap();
        assert_eq!(
            buffer,
            vec![
                0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255,
                255,
            ]
        );
    }
}
