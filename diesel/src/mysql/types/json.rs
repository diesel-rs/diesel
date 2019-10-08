use deserialize::{self, FromSql};
use mysql::{Mysql, MysqlValue};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::*;
use std::io::Write;

impl FromSql<Json, Mysql> for serde_json::Value {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        serde_json::from_slice(value.as_bytes()).map_err(Into::into)
    }
}

impl ToSql<Json, Mysql> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}
