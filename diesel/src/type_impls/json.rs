#![allow(dead_code)]

use crate::deserialize::FromSqlRow;
use crate::expression::AsExpression;
use crate::sql_types::Json;
#[cfg(any(feature = "postgres_backend", feature = "sqlite"))]
use crate::sql_types::Jsonb;

#[derive(AsExpression, FromSqlRow)]
#[diesel(foreign_derive)]
#[diesel(sql_type = Json)]
#[cfg_attr(any(feature = "postgres_backend", feature = "sqlite"), diesel(sql_type = Jsonb))]
struct SerdeJsonValueProxy(serde_json::Value);
