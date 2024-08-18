use crate::model::protocol_type::{PgProtocolType, ProtocolType};
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::{Integer, Record, Text};
use diesel::{deserialize, serialize};

#[derive(Debug, Clone, FromSqlRow, AsExpression, PartialEq, Eq)]
#[diesel(sql_type=crate::schema::smdb::sql_types::ServiceEndpoint)]
pub struct Endpoint {
    pub name: String,
    pub version: i32,
    pub base_uri: String,
    pub port: i32,
    pub protocol: ProtocolType,
}

impl Endpoint {
    pub fn new(
        name: String,
        version: i32,
        base_uri: String,
        port: i32,
        protocol: ProtocolType,
    ) -> Self {
        Self {
            name,
            version,
            base_uri,
            port,
            protocol,
        }
    }
}

impl ToSql<crate::schema::smdb::sql_types::ServiceEndpoint, Pg> for Endpoint {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        serialize::WriteTuple::<(Text, Integer, Text, Integer, PgProtocolType)>::write_tuple(
            &(
                self.name.to_owned(),
                self.version.to_owned(),
                self.base_uri.to_owned(),
                self.port.to_owned(),
                self.protocol.to_owned(),
            ),
            &mut out.reborrow(),
        )
    }
}

impl FromSql<crate::schema::smdb::sql_types::ServiceEndpoint, Pg> for Endpoint {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let (name, version, base_uri, port, protocol) =
            FromSql::<Record<(Text, Integer, Text, Integer, PgProtocolType)>, Pg>::from_sql(bytes)?;

        Ok(Endpoint {
            name,
            version,
            base_uri,
            port,
            protocol,
        })
    }
}
