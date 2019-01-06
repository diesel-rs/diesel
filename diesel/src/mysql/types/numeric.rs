#[cfg(feature = "bigdecimal")]
pub mod bigdecimal {
    extern crate bigdecimal;

    use self::bigdecimal::BigDecimal;
    use std::io::prelude::*;

    use crate::backend::Backend;
    use crate::deserialize::{self, FromSql};
    use crate::mysql::Mysql;
    use crate::serialize::{self, IsNull, Output, ToSql};
    use crate::sql_types::{Binary, Numeric};

    impl ToSql<Numeric, Mysql> for BigDecimal {
        fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
            write!(out, "{}", *self)
                .map(|_| IsNull::No)
                .map_err(|e| e.into())
        }
    }

    impl FromSql<Numeric, Mysql> for BigDecimal {
        fn from_sql(bytes: Option<&<Mysql as Backend>::RawValue>) -> deserialize::Result<Self> {
            let bytes_ptr = <*const [u8] as FromSql<Binary, Mysql>>::from_sql(bytes)?;
            let bytes = unsafe { &*bytes_ptr };
            BigDecimal::parse_bytes(bytes, 10)
                .ok_or_else(|| Box::from(format!("{:?} is not valid decimal number ", bytes)))
        }
    }
}
