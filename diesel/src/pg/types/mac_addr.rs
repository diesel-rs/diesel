use std::io::prelude::*;

use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::MacAddr;

#[allow(dead_code)]
mod foreign_derives {
    use super::*;
    use crate::deserialize::FromSqlRow;
    use crate::expression::AsExpression;

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = MacAddr)]
    struct ByteArrayProxy([u8; 6]);
}

#[cfg(feature = "postgres_backend")]
impl FromSql<MacAddr, Pg> for [u8; 6] {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        value
            .as_bytes()
            .try_into()
            .map_err(|_| "invalid network address format: input isn't 6 bytes.".into())
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<MacAddr, Pg> for [u8; 6] {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(&self[..])
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn macaddr_roundtrip() {
    use crate::query_builder::bind_collector::ByteWrapper;

    let mut buffer = Vec::new();
    let mut bytes = Output::test(ByteWrapper(&mut buffer));
    let input_address = [0x52, 0x54, 0x00, 0xfb, 0xc6, 0x16];
    ToSql::<MacAddr, Pg>::to_sql(&input_address, &mut bytes).unwrap();
    let output_address: [u8; 6] = FromSql::from_sql(PgValue::for_test(&buffer)).unwrap();
    assert_eq!(input_address, output_address);
}
