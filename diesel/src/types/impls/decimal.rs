#[cfg(feature="bigdecimal")]
mod bigdecimal {
    extern crate bigdecimal;
    expression_impls! {
        Numeric -> bigdecimal::BigDecimal,
    }

    queryable_impls! {
        Numeric -> bigdecimal::BigDecimal,
    }
}
