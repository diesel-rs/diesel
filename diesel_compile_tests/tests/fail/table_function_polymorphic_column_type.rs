extern crate diesel;

use diesel::prelude::*;
use diesel::query_source::SingleColumnRelation;
use diesel::sql_types::Integer;

view! {
    text_series {
        value -> Text,
    }
}

#[diesel::declare_sql_function]
extern "SQL" {
    #[query_source(Relation, base_type)]
    fn generate_series<
        Relation: SingleColumnRelation<ColumnSqlType = Value>,
        Value: diesel::sql_types::SqlType + diesel::expression::TypedExpressionType,
    >(
        start: Value,
        stop: Value,
    ) -> Relation::SqlType;
}

fn main() {
    let _ = generate_series::<text_series::view, Integer, _, _>(1, 3);
    //~^ ERROR: type mismatch resolving `<view as SingleColumnRelation>::ColumnSqlType == Integer`
}
