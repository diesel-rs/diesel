#[cfg(feature = "uuid")]
mod uuid {
    extern crate uuid;
    use std::io::prelude::*;

    use crate::deserialize::{self, FromSql, FromSqlRow};
    use crate::expression::AsExpression;
    use crate::serialize::{self, IsNull, Output, ToSql};
    use crate::sql_types::Uuid;

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Uuid)]
    #[allow(dead_code)]
    struct UuidProxy(uuid::Uuid);

    #[cfg(feature = "postgres_backend")]
    impl FromSql<Uuid, crate::pg::Pg> for uuid::Uuid {
        fn from_sql(value: crate::pg::PgValue<'_>) -> deserialize::Result<Self> {
            uuid::Uuid::from_slice(value.as_bytes()).map_err(Into::into)
        }
    }

    #[cfg(feature = "postgres_backend")]
    impl ToSql<Uuid, crate::pg::Pg> for uuid::Uuid {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, crate::pg::Pg>) -> serialize::Result {
            out.write_all(self.as_bytes())
                .map(|_| IsNull::No)
                .map_err(Into::into)
        }
    }

    #[cfg(feature = "sqlite")]
    impl ToSql<Uuid, crate::sqlite::Sqlite> for uuid::Uuid {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, crate::sqlite::Sqlite>) -> serialize::Result {
            out.set_value(self.as_bytes().as_slice());
			Ok(IsNull::No)
        }
    }

    #[cfg(feature = "sqlite")]
    impl FromSql<Uuid, crate::sqlite::Sqlite> for uuid::Uuid {
        fn from_sql(mut value: crate::sqlite::SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
            uuid::Uuid::from_slice(value.read_blob()).map_err(Into::into)
        }
    }

    #[cfg(test)]
	#[cfg(feature = "postgres_backend")]
    mod postgres_tests {
        use super::*;

        #[diesel_test_helper::test]
        fn uuid_to_sql() {
            use crate::query_builder::bind_collector::ByteWrapper;

            let mut buffer = Vec::new();
            let bytes = [
                0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65,
                0x66, 0x31, 0x32,
            ];

            let test_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
            let mut bytes = Output::test(ByteWrapper(&mut buffer));
            ToSql::<Uuid, crate::pg::Pg>::to_sql(&test_uuid, &mut bytes).unwrap();
            assert_eq!(&buffer, test_uuid.as_bytes());
        }

        #[diesel_test_helper::test]
        fn some_uuid_from_sql() {
            let bytes = [
                0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65,
                0x66, 0x31, 0x32,
            ];
            let input_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
            let output_uuid = <uuid::Uuid as FromSql<Uuid, crate::pg::Pg>>::from_sql(
                crate::pg::PgValue::for_test(input_uuid.as_bytes()),
            )
            .unwrap();
            assert_eq!(input_uuid, output_uuid);
        }

        #[diesel_test_helper::test]
        fn bad_uuid_from_sql() {
            let uuid = uuid::Uuid::from_sql(crate::pg::PgValue::for_test(b"boom"));
            assert!(uuid.is_err());
            // The error message changes slightly between different
            // uuid versions, so we just check on the relevant parts
            // The exact error messages are either:
            // "invalid bytes length: expected 16, found 4"
            // or
            // "invalid length: expected 16 bytes, found 4"
            let error_message = uuid.unwrap_err().to_string();
            assert!(error_message.starts_with("invalid"));
            assert!(error_message.contains("length"));
            assert!(error_message.contains("expected 16"));
            assert!(error_message.ends_with("found 4"));
        }

        #[diesel_test_helper::test]
        fn no_uuid_from_sql() {
            let uuid = uuid::Uuid::from_nullable_sql(None);
            assert_eq!(
                uuid.unwrap_err().to_string(),
                "Unexpected null for non-null column"
            );
        }
    }

	#[cfg(test)]
	#[cfg(feature = "sqlite")]
	mod sqlite_tests {
		use super::*;

		#[diesel_test_helper::test]
		fn uuid_to_sql() {
			use crate::query_builder::bind_collector::ByteWrapper;

			let mut buffer = Vec::new();
			let bytes = [
				0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65,
				0x66, 0x31, 0x32,
			];

			let test_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
			let mut bytes = Output::test(ByteWrapper(&mut buffer));
			ToSql::<Uuid, crate::sqlite::Sqlite>::to_sql(&test_uuid, &mut bytes).unwrap();
			assert_eq!(&buffer, test_uuid.as_bytes());
		}

		#[diesel_test_helper::test]
		fn some_uuid_from_sql() {
			let bytes = [
				0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65,
				0x66, 0x31, 0x32,
			];
			let input_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
			let output_uuid = <uuid::Uuid as FromSql<Uuid, crate::sqlite::Sqlite>>::from_sql(
				crate::sqlite::SqliteValue::for_test(&bytes),
			)
			.unwrap();
			assert_eq!(input_uuid, output_uuid);
		}

		#[diesel_test_helper::test]
		fn bad_uuid_from_sql() {
			let uuid = uuid::Uuid::from_sql(crate::sqlite::SqliteValue::for_test(b"boom"));
			assert!(uuid.is_err());
			// The error message changes slightly between different
			// uuid versions, so we just check on the relevant parts
			// The exact error messages are either:
			// "invalid bytes length: expected 16, found 4"
			// or
			// "invalid length: expected 16 bytes, found 4"
			let error_message = uuid.unwrap_err().to_string();
			assert!(error_message.starts_with("invalid"));
			assert!(error_message.contains("length"));
			assert!(error_message.contains("expected 16"));
			assert!(error_message.ends_with("found 4"));
		}
	}
}
