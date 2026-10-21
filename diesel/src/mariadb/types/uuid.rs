use std::io::prelude::Write;

use crate::deserialize::FromSql;
use crate::mariadb::Mariadb;
use crate::serialize::{IsNull, ToSql};
use crate::sql_types::{Text, Uuid};

impl FromSql<Uuid, Mariadb> for uuid::Uuid {
    fn from_sql(
        bytes: <Mariadb as crate::backend::Backend>::RawValue<'_>,
    ) -> crate::deserialize::Result<Self> {
        let uuid_rfc = <String as FromSql<Text, Mariadb>>::from_sql(bytes)?;
        uuid::Uuid::parse_str(&uuid_rfc).map_err(Into::into)
    }
}

impl ToSql<Uuid, Mariadb> for uuid::Uuid {
    fn to_sql<'b>(
        &'b self,
        out: &mut crate::serialize::Output<'b, '_, Mariadb>,
    ) -> crate::serialize::Result {
        write!(out, "{}", self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;
    use crate::mariadb::MariadbValue;
    use crate::mysql_like::MysqlType;
    use crate::serialize::Output;
    use crate::test_helpers::format_error;

    #[diesel_test_helper::test]
    fn uuid_to_sql() {
        use crate::query_builder::bind_collector::ByteWrapper;

        let mut buffer = Vec::new();

        let hyphenated = "ffffffff-ffff-ffff-6162-636465663132";

        let test_uuid = uuid::Uuid::from_str(hyphenated).unwrap();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        ToSql::<Uuid, Mariadb>::to_sql(&test_uuid, &mut bytes).unwrap();
        assert_eq!(&buffer, hyphenated.as_bytes());
    }

    #[diesel_test_helper::test]
    fn some_uuid_from_sql() {
        let bytes = [
            0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
            0x31, 0x32,
        ];
        let input_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
        let output_uuid =
            <uuid::Uuid as FromSql<Uuid, Mariadb>>::from_sql(MariadbValue::new_internal(
                input_uuid.hyphenated().to_string().as_bytes(),
                MysqlType::String,
            ))
            .unwrap();
        assert_eq!(input_uuid, output_uuid);
    }

    #[diesel_test_helper::test]
    fn bad_uuid_from_sql() {
        let uuid = uuid::Uuid::from_sql(MariadbValue::new_internal(b"boom", MysqlType::String));
        assert!(uuid.is_err());
        // The error message changes slightly between different
        // uuid versions, so we just check on the relevant parts
        // The exact error messages are either:
        // "invalid character: found `o` at 1"
        // or
        // "invalid length: expected 16 bytes, found 4"
        let error_message = format_error(&*uuid.unwrap_err());
        assert!(error_message.starts_with("invalid"));
        assert!(error_message.contains("character"));
        assert!(error_message.contains("o"));
        assert!(error_message.ends_with("at 1"));
    }

    #[diesel_test_helper::test]
    fn no_uuid_from_sql() {
        let uuid = uuid::Uuid::from_nullable_sql(None);
        assert_eq!(
            format_error(&*uuid.unwrap_err()),
            "Unexpected null for non-null column"
        );
    }
}
