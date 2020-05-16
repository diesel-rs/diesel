use crate::mysql::{Mysql, MysqlValue};
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use std::io::prelude::*;

impl FromSql<sql_types::Json, Mysql> for serde_json::Value {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        serde_json::from_slice(value.as_bytes()).map_err(|_| "Invalid Json".into())
    }
}

impl ToSql<sql_types::Json, MysqlValue> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlValue>) -> serialize::Result {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}
