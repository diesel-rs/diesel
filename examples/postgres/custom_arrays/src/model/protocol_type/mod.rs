use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sql_types::SqlType;
use diesel::{deserialize, serialize};
use std::io::Write;

//  Diesel type mapping requires a struct to derive a SqlType for custom ToSql and FromSql implementations
#[derive(SqlType)]
#[diesel(sql_type = protocol_type)]
#[diesel(postgres_type(name = "protocol_type", schema = "smdb"))]
pub struct PgProtocolType;

#[derive(Debug, Clone, FromSqlRow, AsExpression, PartialEq, Eq)]
#[diesel(sql_type = PgProtocolType)]
#[allow(clippy::upper_case_acronyms)]
pub enum ProtocolType {
    UnknownProtocol,
    GRPC,
    HTTP,
    UDP,
}

impl ToSql<PgProtocolType, Pg> for ProtocolType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            ProtocolType::UnknownProtocol => out.write_all(b"UnknownProtocol")?,
            ProtocolType::GRPC => out.write_all(b"GRPC")?,
            ProtocolType::HTTP => out.write_all(b"HTTP")?,
            ProtocolType::UDP => out.write_all(b"UDP")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<PgProtocolType, Pg> for ProtocolType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"UnknownProtocol" => Ok(ProtocolType::UnknownProtocol),
            b"GRPC" => Ok(ProtocolType::GRPC),
            b"HTTP" => Ok(ProtocolType::HTTP),
            b"UDP" => Ok(ProtocolType::UDP),
            _ => Err(DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new(format!(
                    "Unrecognized enum variant: {:?}",
                    String::from_utf8_lossy(bytes.as_bytes())
                )),
            )
            .into()),
        }
    }
}
