#[cfg(feature = "bigdecimal")]
pub mod bigdecimal {
    extern crate bigdecimal;

    use self::bigdecimal::{BigDecimal, FromPrimitive};
    use std::io::prelude::*;

    use crate::deserialize::{self, FromSql};
    use crate::mysql::{Mysql, MysqlValue, NumericRepresentation};
    use crate::serialize::{self, IsNull, Output, ToSql};
    use crate::sql_types::Numeric;

    impl ToSql<Numeric, Mysql> for BigDecimal {
        fn to_sql<'a: 'b, 'b>(&'a self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
            write!(out, "{}", *self)
                .map(|_| IsNull::No)
                .map_err(Into::into)
        }
    }

    impl FromSql<Numeric, Mysql> for BigDecimal {
        fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
            match value.numeric_value()? {
                NumericRepresentation::Tiny(x) => Ok(x.into()),
                NumericRepresentation::Small(x) => Ok(x.into()),
                NumericRepresentation::Medium(x) => Ok(x.into()),
                NumericRepresentation::Big(x) => Ok(x.into()),
                NumericRepresentation::Float(x) => BigDecimal::from_f32(x)
                    .ok_or_else(|| format!("{} is not valid decimal number ", x).into()),
                NumericRepresentation::Double(x) => BigDecimal::from_f64(x)
                    .ok_or_else(|| format!("{} is not valid decimal number ", x).into()),
                NumericRepresentation::Decimal(bytes) => BigDecimal::parse_bytes(bytes, 10)
                    .ok_or_else(|| format!("{:?} is not valid decimal number ", bytes).into()),
            }
        }
    }
}
