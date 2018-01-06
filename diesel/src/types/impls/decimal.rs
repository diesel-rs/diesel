#![allow(dead_code)]

#[cfg(feature = "bigdecimal")]
mod bigdecimal {
    extern crate bigdecimal;
    use self::bigdecimal::BigDecimal;
    use types::Numeric;

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Numeric"]
    struct BigDecimalProxy(BigDecimal);
}
