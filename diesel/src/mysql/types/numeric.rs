#[cfg(feature = "bigdecimal")]
pub mod bigdecimal {
    extern crate bigdecimal;

    use self::bigdecimal::BigDecimal;
    use std::io::prelude::*;

    use crate::deserialize::{self, FromSql};
    use crate::mysql::{Mysql, MysqlValue};
    use crate::serialize::{self, IsNull, Output, ToSql};
    use crate::sql_types::Numeric;

    impl ToSql<Numeric, Mysql> for BigDecimal {
        fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
            write!(out, "{}", *self)
                .map(|_| IsNull::No)
                .map_err(Into::into)
        }
    }

    impl FromSql<Numeric, Mysql> for BigDecimal {
        fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
            use crate::mysql::NumericRepresentation::*;
            let data = not_none!(value);
            match data.numeric_value()? {
                Tiny(x) => Ok(x.into()),
                Small(x) => Ok(x.into()),
                Medium(x) => Ok(x.into()),
                Big(x) => Ok(x.into()),
                Float(x) => Ok(x.into()),
                Double(x) => Ok(x.into()),
                Decimal(bytes) => BigDecimal::parse_bytes(bytes, 10)
                    .ok_or_else(|| format!("{:?} is not valid decimal number ", bytes).into()),
            }
        }
    }
}
