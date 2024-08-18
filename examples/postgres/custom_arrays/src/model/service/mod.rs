use crate::model::endpoint_type::Endpoint;
use diesel::{AsChangeset, Insertable, Queryable, Selectable};

mod service_impl;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name= crate::schema::smdb::service,  primary_key(service_id))]
pub struct Service {
    pub service_id: i32,
    pub name: String,
    pub version: i32,
    pub online: bool,
    pub description: String,
    pub health_check_uri: String,
    pub base_uri: String,
    pub dependencies: Vec<Option<i32>>,
    pub endpoints: Vec<Option<Endpoint>>,
}

#[derive(Debug, Clone, Queryable, Insertable)]
#[diesel(table_name= crate::schema::smdb::service,  primary_key(service_id))]
pub struct CreateService {
    pub service_id: i32,
    pub name: String,
    pub version: i32,
    pub online: bool,
    pub description: String,
    pub health_check_uri: String,
    pub base_uri: String,
    pub dependencies: Vec<Option<i32>>,
    pub endpoints: Vec<Option<Endpoint>>,
}

impl CreateService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        service_id: i32,
        name: String,
        version: i32,
        online: bool,
        description: String,
        health_check_uri: String,
        base_uri: String,
        dependencies: Vec<Option<i32>>,
        endpoints: Vec<Option<Endpoint>>,
    ) -> Self {
        Self {
            service_id,
            name,
            version,
            online,
            description,
            health_check_uri,
            base_uri,
            dependencies,
            endpoints,
        }
    }
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset)]
#[diesel(table_name=crate::schema::smdb::service)]
pub struct UpdateService {
    pub name: Option<String>,
    pub version: Option<i32>,
    pub online: Option<bool>,
    pub description: Option<String>,
    pub health_check_uri: Option<String>,
    pub base_uri: Option<String>,
    pub dependencies: Option<Vec<Option<i32>>>,
    pub endpoints: Option<Vec<Option<Endpoint>>>,
}

impl UpdateService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Option<String>,
        version: Option<i32>,
        online: Option<bool>,
        description: Option<String>,
        health_check_uri: Option<String>,
        base_uri: Option<String>,
        dependencies: Option<Vec<Option<i32>>>,
        endpoints: Option<Vec<Option<Endpoint>>>,
    ) -> Self {
        Self {
            name,
            version,
            online,
            description,
            health_check_uri,
            base_uri,
            dependencies,
            endpoints,
        }
    }
}
