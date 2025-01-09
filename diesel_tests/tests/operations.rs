use std::str::FromStr;

use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::{insert_into, prelude::*, update};

diesel::table! {
    bigdecimal_table (id) {
        id -> Integer,
        big_decimal -> Decimal,
    }
}

#[derive(Debug, Insertable, Queryable, Selectable)]
#[diesel(table_name = bigdecimal_table)]
struct CustomBigDecimal {
    pub id: i32,
    pub big_decimal: BigDecimal,
}

#[td::test]
fn big_decimal_add() {
    let connection = &mut connection();

    let custom_value = CustomBigDecimal {
        id: 1,
        big_decimal: BigDecimal::from_str("0.80").unwrap(),
    };

    diesel::sql_query(
        "
        CREATE TEMPORARY TABLE bigdecimal_table (
            id SERIAL PRIMARY KEY,
            big_decimal DECIMAL(8,2) NOT NULL
            );
        ",
    )
    .execute(connection)
    .unwrap();

    insert_into(bigdecimal_table::table)
        .values(&custom_value)
        .execute(connection)
        .unwrap();

    let val = BigDecimal::from_str("0.1").unwrap();

    update(bigdecimal_table::table)
        .set(bigdecimal_table::big_decimal.eq(bigdecimal_table::big_decimal + val))
        .execute(connection)
        .unwrap();

    let updated: CustomBigDecimal = bigdecimal_table::table.first(connection).unwrap();

    assert_eq!(BigDecimal::from_str("0.90").unwrap(), updated.big_decimal);
}
