// @generated automatically by Diesel CLI.

pub mod smdb {
    pub mod sql_types {
        #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
        #[diesel(postgres_type(name = "service_endpoint", schema = "smdb"))]
        pub struct ServiceEndpoint;
    }

    diesel::table! {
        use diesel::sql_types::*;
        use super::sql_types::ServiceEndpoint;

        smdb.service (service_id) {
            service_id -> Int4,
            name -> Text,
            version -> Int4,
            online -> Bool,
            description -> Text,
            health_check_uri -> Text,
            base_uri -> Text,
            dependencies -> Array<Nullable<Int4>>,
            endpoints -> Array<Nullable<ServiceEndpoint>>,
        }
    }
}
