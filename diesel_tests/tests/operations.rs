use std::str::FromStr;

use crate::diesel::connection::SimpleConnection;
use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::{prelude::*, update};

diesel::table! {
    bigdecimal_table (id) {
        id -> Integer,
        big_decimal -> Decimal,
    }
}

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = bigdecimal_table)]
struct CustomBigDecimal {
    pub id: i32,
    pub big_decimal: BigDecimal,
}

#[test]
fn big_decimal_add() {
    let data = vec![
        CustomBigDecimal {
            id: 1,
            big_decimal: BigDecimal::from_str("0.8").unwrap(),
        },
        CustomBigDecimal {
            id: 2,
            big_decimal: BigDecimal::from_str("0.9").unwrap(),
        },
    ];

    let connection = &mut connection();
    connection
        .batch_execute(
            r#"
        CREATE TEMPORARY TABLE bigdecimal_table (
            id SERIAL PRIMARY KEY,
            big_decimal DECIMAL NOT NULL
            );
        "#,
        )
        .unwrap();

    diesel::insert_into(bigdecimal_table::table)
        .values(&data)
        .execute(connection)
        .unwrap();

    let val = BigDecimal::from_str("0.1").unwrap();
    let updated = update(bigdecimal_table::table)
        .set(bigdecimal_table::big_decimal.eq(bigdecimal_table::big_decimal + val));
}
