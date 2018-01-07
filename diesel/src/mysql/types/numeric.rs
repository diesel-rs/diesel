#[cfg(feature = "bigdecimal")]
pub mod bigdecimal {
    extern crate bigdecimal;

    use std::error::Error;
    use std::io::prelude::*;

    use mysql::Mysql;

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
        fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
            let bytes = not_none!(bytes);
            BigDecimal::parse_bytes(bytes, 10)
                .ok_or_else(|| Box::from(format!("{:?} is not valid decimal number ", bytes)))
        }
    }
}
