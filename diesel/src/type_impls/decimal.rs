#![allow(dead_code)]

#[cfg(feature = "numeric")]
mod bigdecimal {
    extern crate bigdecimal;
    use self::bigdecimal::BigDecimal;
    use crate::deserialize::FromSqlRow;
    use crate::expression::AsExpression;
    use crate::sql_types::Numeric;

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Numeric)]
    struct BigDecimalProxy(BigDecimal);
}
