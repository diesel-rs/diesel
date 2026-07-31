// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(Clone, diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "some_enum"))]
    #[diesel(mysql_type(name = "Enum"))]
    #[diesel(enum_type)]
    pub struct SomeEnum;

    #[derive(Clone, diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "some_enum2"))]
    #[diesel(mysql_type(name = "Enum"))]
    #[diesel(enum_type)]
    pub struct SomeEnum2;
}

pub mod rust_types {
    #[derive(Debug, diesel::types::Enum)]
    #[diesel(sql_type = super::sql_types::SomeEnum)]
    pub enum SomeEnum {
        #[diesel(rename = "a")]
        A,
        #[diesel(rename = "b")]
        B,
        #[diesel(rename = "c")]
        C,
        #[diesel(rename = "d'd")]
        D,
    }

    #[derive(Debug, diesel::types::Enum)]
    #[diesel(sql_type = super::sql_types::SomeEnum2)]
    #[diesel(rename_all = "UPPERCASE")]
    pub enum SomeEnum2 {
        FooBar,
        BazBoom,
        Bazz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::SomeEnum;
    use super::sql_types::SomeEnum2;
    resource (resource_id) {
        resource_id -> Int4,
        some_field -> SomeEnum,
        some_field2 -> SomeEnum2,
    }
}
