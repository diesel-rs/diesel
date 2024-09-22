use crate::model::endpoint_type::Endpoint;
use crate::model::service::{CreateService, Service, UpdateService};
use crate::schema::smdb::service::dsl::*;
use crate::Connection;
use diesel::{
    insert_into, ExpressionMethods, OptionalExtension, QueryDsl, QueryResult, RunQueryDsl,
    SelectableHelper,
};

impl Service {
    pub fn create(db: &mut Connection, item: &CreateService) -> QueryResult<Self> {
        insert_into(crate::schema::smdb::service::table)
            .values(item)
            .get_result::<Service>(db)
    }

    pub fn count(db: &mut Connection) -> QueryResult<i64> {
        service.count().get_result::<i64>(db)
    }

    pub fn count_u64(db: &mut Connection) -> QueryResult<u64> {
        service.count().get_result::<i64>(db).map(|c| c as u64)
    }

    pub fn check_if_service_id_exists(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<bool> {
        service
            .find(param_service_id)
            .first::<Service>(db)
            .optional()
            .map(|arg0: Option<Service>| Option::is_some(&arg0))
    }

    pub fn check_all_services_online(
        db: &mut Connection,
        services: &[i32],
    ) -> QueryResult<(bool, Option<String>)> {
        for id in services {
            if !Service::check_if_service_id_online(db, *id)? {
                return Ok((false, Some(format!("Service {} is offline", id))));
            }
        }

        Ok((true, None))
    }

    pub fn check_if_service_id_online(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<bool> {
        service
            .filter(service_id.eq(param_service_id))
            .select(online)
            .first::<bool>(db)
    }

    pub fn get_all_online_services(db: &mut Connection) -> QueryResult<Vec<Self>> {
        service
            .filter(online.eq(true))
            .select(Service::as_returning())
            .load::<Service>(db)
    }

    pub fn get_all_offline_services(db: &mut Connection) -> QueryResult<Vec<Self>> {
        service
            .filter(online.eq(false))
            .select(Service::as_returning())
            .load::<Service>(db)
    }

    pub fn get_all_service_dependencies(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<Vec<Option<i32>>> {
        service
            .filter(service_id.eq(param_service_id))
            .select(dependencies)
            .first::<Vec<Option<i32>>>(db)
    }

    pub fn get_all_service_endpoints(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<Vec<Option<Endpoint>>> {
        service
            .filter(service_id.eq(param_service_id))
            .select(endpoints)
            .first::<Vec<Option<Endpoint>>>(db)
    }

    pub fn read(db: &mut Connection, param_service_id: i32) -> QueryResult<Self> {
        service
            .filter(service_id.eq(param_service_id))
            .first::<Service>(db)
    }

    pub fn read_all(db: &mut Connection) -> QueryResult<Vec<Self>> {
        service.load::<Service>(db)
    }

    pub fn set_service_online(db: &mut Connection, param_service_id: i32) -> QueryResult<()> {
        Self::set_svc_online(db, param_service_id, true)
    }

    pub fn set_service_offline(db: &mut Connection, param_service_id: i32) -> QueryResult<()> {
        Self::set_svc_online(db, param_service_id, false)
    }

    fn set_svc_online(
        db: &mut Connection,
        param_service_id: i32,
        param_online: bool,
    ) -> QueryResult<()> {
        diesel::update(service.filter(service_id.eq(param_service_id)))
            .set(online.eq(param_online))
            .execute(db)?;
        Ok(())
    }

    pub fn update(
        db: &mut Connection,
        param_service_id: i32,
        item: &UpdateService,
    ) -> QueryResult<Self> {
        diesel::update(service.filter(service_id.eq(param_service_id)))
            .set(item)
            .returning(Service::as_returning())
            .get_result(db)
    }

    pub fn delete(db: &mut Connection, param_service_id: i32) -> QueryResult<usize> {
        diesel::delete(service.filter(service_id.eq(param_service_id))).execute(db)
    }
}
