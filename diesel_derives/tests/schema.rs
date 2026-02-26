table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
        r#type -> Nullable<Text>,
    }
}

table! {
    users_ {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
        r#type -> Nullable<Text>,
    }
}

pub mod sql_types {
    #[cfg(feature = "postgres")]
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "color"))]
    pub struct Color;

    #[cfg(feature = "mysql")]
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(mysql_type(name = "Enum"))]
    pub struct CarsPaintColorEnum;
}

#[cfg(feature = "postgres")]
table! {
    use diesel::sql_types::*;
    use super::sql_types::Color;

    cars {
        id -> Integer,
        paint_color -> Color
    }
}

#[cfg(feature = "mysql")]
table! {
    use diesel::sql_types::*;
    use super::sql_types::CarsPaintColorEnum;

    cars {
        id -> Integer,
        paint_color -> CarsPaintColorEnum
    }
}
