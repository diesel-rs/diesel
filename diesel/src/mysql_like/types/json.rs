use crate::deserialize::{self, FromSql};
use crate::mysql_like::{MysqlLikeBackend, MysqlValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

#[cfg(feature = "serde_json")]
impl<DB: MysqlLikeBackend> FromSql<sql_types::Json, DB> for serde_json::Value {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        serde_json::from_slice(value.as_bytes()).map_err(|_| "Invalid Json".into())
    }
}

#[cfg(feature = "serde_json")]
impl<DB: MysqlLikeBackend> ToSql<sql_types::Json, DB> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(all(test, feature = "mysql"))]
mod tests {
    use super::*;
    use crate::mysql::Mysql;
    use crate::test_helpers::format_error;

    #[diesel_test_helper::test]
    fn regression_json_float_survives_a_round_trip() {
        use crate::mysql::MysqlType;

        crate::test_helpers::assert_floats_survive_a_json_round_trip::<sql_types::Json, Mysql>(
            |bytes| {
                FromSql::<sql_types::Json, Mysql>::from_sql(MysqlValue::new_internal(
                    bytes,
                    MysqlType::String,
                ))
            },
        );
    }

    #[diesel_test_helper::test]
    fn json_to_sql() {
        use crate::query_builder::bind_collector::ByteWrapper;

        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json = serde_json::Value::Bool(true);
        ToSql::<sql_types::Json, Mysql>::to_sql(&test_json, &mut bytes).unwrap();
        assert_eq!(buffer, b"true");
    }

    #[diesel_test_helper::test]
    fn some_json_from_sql() {
        use crate::mysql::MysqlType;
        let input_json = b"true";
        let output_json: serde_json::Value = FromSql::<sql_types::Json, Mysql>::from_sql(
            MysqlValue::new_internal(input_json, MysqlType::String),
        )
        .unwrap();
        assert_eq!(output_json, serde_json::Value::Bool(true));
    }

    #[diesel_test_helper::test]
    fn bad_json_from_sql() {
        use crate::mysql::MysqlType;
        let uuid: Result<serde_json::Value, _> = FromSql::<sql_types::Json, Mysql>::from_sql(
            MysqlValue::new_internal(b"boom", MysqlType::String),
        );
        assert_eq!(format_error(&*uuid.unwrap_err()), "Invalid Json");
    }

    #[diesel_test_helper::test]
    fn no_json_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Json, Mysql>::from_nullable_sql(None);
        assert_eq!(
            format_error(&*uuid.unwrap_err()),
            "Unexpected null for non-null column"
        );
    }
}
