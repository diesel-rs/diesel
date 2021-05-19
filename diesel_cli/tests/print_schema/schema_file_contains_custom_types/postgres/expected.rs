// @generated automatically by Diesel CLI.

#[derive(diesel::SqlType)]
#[postgres(type_name = "my_type")]
pub struct MyType;

diesel::table! {
    use diesel::sql_types::*;
    use super::MyType;

    custom_types (id) {
        id -> Int4,
        custom_enum -> MyType,
    }
}
