#[cfg(feature = "bigdecimal")]
pub mod bigdecimal {
    extern crate bigdecimal;

    use std::error::Error;
    use std::io::prelude::*;

    use mysql::{Mysql, MysqlValue};

    use self::bigdecimal::BigDecimal;

    use types::{self, FromSql, IsNull, ToSql, ToSqlOutput};

    impl ToSql<types::Numeric, Mysql> for BigDecimal {
        fn to_sql<W: Write>(
            &self,
            out: &mut ToSqlOutput<W, Mysql>,
        ) -> Result<IsNull, Box<Error + Send + Sync>> {
            write!(out, "{}", *self)
                .map(|_| IsNull::No)
                .map_err(|e| e.into())
        }
    }

    impl FromSql<types::Numeric, Mysql> for BigDecimal {
        fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
            use mysql::NumericRepresentation::*;

            let data = not_none!(value).numeric_value()?;
            match data {
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
