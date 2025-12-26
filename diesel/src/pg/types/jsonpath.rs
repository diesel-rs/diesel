//! Support for the `jsonpath` type under PostgreSQL.

use std::io::prelude::*;

use crate::deserialize::{self, FromSql};
use crate::pg::sql_types::Jsonpath;
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};

#[cfg(feature = "postgres_backend")]
impl FromSql<Jsonpath, Pg> for String {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        String::from_utf8(value.as_bytes().to_vec()).map_err(Into::into)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<Jsonpath, Pg> for String {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<Jsonpath, Pg> for str {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())?;
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_builder::bind_collector::ByteWrapper;

    #[diesel_test_helper::test]
    fn jsonpath_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_path = "$.foo.bar";
        ToSql::<Jsonpath, Pg>::to_sql(&test_path, &mut bytes).unwrap();
        assert_eq!(buffer, b"$.foo.bar");
    }

    #[diesel_test_helper::test]
    fn jsonpath_from_sql() {
        let input = b"$.foo.bar";
        let output: String = FromSql::<Jsonpath, Pg>::from_sql(PgValue::for_test(input)).unwrap();
        assert_eq!(output, "$.foo.bar");
    }

    #[diesel_test_helper::test]
    fn no_jsonpath_from_sql() {
        let result: Result<String, _> = FromSql::<Jsonpath, Pg>::from_nullable_sql(None);
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }
}
