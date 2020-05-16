#![allow(dead_code)]

use crate::deserialize::FromSqlRow;
use crate::expression::AsExpression;
use crate::sql_types::Json;
#[cfg(feature = "postgres")]
use crate::sql_types::Jsonb;

#[derive(FromSqlRow, AsExpression)]
#[diesel(foreign_derive)]
#[sql_type = "Json"]
#[cfg_attr(feature = "postgres", sql_type = "Jsonb")]
struct SerdeJsonValueProxy(serde_json::Value);
