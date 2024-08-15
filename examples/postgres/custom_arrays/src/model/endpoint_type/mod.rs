use crate::model::protocol_type::ProtocolType;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;

mod endpoint_type_impl;

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
