#![allow(dead_code)]

#[cfg(feature = "bigdecimal")]
mod bigdecimal {
    extern crate bigdecimal;
    use self::bigdecimal::BigDecimal;
    use types::Numeric;

    expression_impls!(Numeric -> BigDecimal);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct BigDecimalProxy(BigDecimal);
}
