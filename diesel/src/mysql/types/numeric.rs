#[cfg(feature = "numeric")]
mod bigdecimal {
    use bigdecimal::{BigDecimal, FromPrimitive};
    use std::io::prelude::*;

    use crate::deserialize::{self, FromSql};
    use crate::mysql::{Mysql, MysqlValue, NumericRepresentation};
    use crate::serialize::{self, IsNull, Output, ToSql};
    use crate::sql_types::Numeric;

    #[cfg(all(feature = "mysql_backend", feature = "numeric"))]
    impl ToSql<Numeric, Mysql> for BigDecimal {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
            write!(out, "{}", *self)
                .map(|_| IsNull::No)
                .map_err(Into::into)
        }
    }

    #[cfg(all(feature = "mysql_backend", feature = "numeric"))]
    impl FromSql<Numeric, Mysql> for BigDecimal {
        fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
            match value.numeric_value()? {
                NumericRepresentation::Tiny(x) => Ok(x.into()),
                NumericRepresentation::Small(x) => Ok(x.into()),
                NumericRepresentation::Medium(x) => Ok(x.into()),
                NumericRepresentation::Big(x) => Ok(x.into()),
                NumericRepresentation::Float(x) => BigDecimal::from_f32(x)
                    .ok_or_else(|| format!("{x} is not valid decimal number ").into()),
                NumericRepresentation::Double(x) => BigDecimal::from_f64(x)
                    .ok_or_else(|| format!("{x} is not valid decimal number ").into()),
                NumericRepresentation::Decimal(bytes) => BigDecimal::parse_bytes(bytes, 10)
                    .ok_or_else(|| format!("{bytes:?} is not valid decimal number ").into()),
            }
        }
    }
}
