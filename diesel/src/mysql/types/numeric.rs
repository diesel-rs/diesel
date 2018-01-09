#[cfg(feature = "bigdecimal")]
pub mod bigdecimal {
    extern crate bigdecimal;

    use self::bigdecimal::BigDecimal;
    use std::io::prelude::*;

    use deserialize::{self, FromSql};
    use mysql::Mysql;
    use serialize::{self, IsNull, Output, ToSql};
    use sql_types::Numeric;

    impl ToSql<Numeric, Mysql> for BigDecimal {
        fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
            write!(out, "{}", *self)
                .map(|_| IsNull::No)
                .map_err(|e| e.into())
        }
    }

    impl FromSql<Numeric, Mysql> for BigDecimal {
        fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
            let bytes = not_none!(bytes);
            BigDecimal::parse_bytes(bytes, 10)
                .ok_or_else(|| Box::from(format!("{:?} is not valid decimal number ", bytes)))
        }
    }
}
