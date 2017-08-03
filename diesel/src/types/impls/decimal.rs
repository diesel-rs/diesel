#[cfg(feature="bigdecimal")]
mod bigdecimal {
    extern crate bigdecimal;
    use types::Numeric;

    expression_impls!(Numeric -> bigdecimal::BigDecimal);
    queryable_impls!(Numeric -> bigdecimal::BigDecimal);
}
