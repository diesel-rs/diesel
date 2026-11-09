use diesel_derives::{AsExpression, FromSqlRow};

use crate::sql_types::Uuid;

#[derive(AsExpression, FromSqlRow)]
#[diesel(foreign_derive)]
#[diesel(sql_type = Uuid)]
#[allow(dead_code)]
struct UuidProxy(uuid::Uuid);
