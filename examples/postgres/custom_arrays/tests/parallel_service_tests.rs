use custom_arrays::model::endpoint_type::Endpoint;
use custom_arrays::model::protocol_type::ProtocolType;
use custom_arrays::model::service;
use custom_arrays::model::service::{CreateService, UpdateService};
use custom_arrays::Connection;
use diesel::{Connection as DieselConnection, PgConnection};
use dotenvy::dotenv;
use std::env;

fn run_db_migration(conn: &mut Connection) {
    println!("run_db_migration");
    let res = custom_arrays::run_db_migration(conn);
    assert!(res.is_ok(), "{:?}", res.unwrap_err());
}

fn postgres_connection() -> PgConnection {
    println!("postgres_connection");
    dotenv().ok();

    let database_url = env::var("PG_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("PG_DATABASE_URL must be set");

    let mut conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Failed to connect, error: {e}"));
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");
    run_db_migration(&mut conn);
    conn
}

#[test]
fn test_create_service() {
    let connection = &mut postgres_connection();

    let service = get_crate_service();
    let endpoints = get_endpoints();
    let dependencies = get_dependencies();

    let result = service::Service::create(connection, &service);

    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service = result.unwrap();

    assert_eq!(service.service_id, 1);
    assert_eq!(service.name, "test");
    assert_eq!(service.version, 1);
    assert!(service.online);
    assert_eq!(service.description, "test");
    assert_eq!(service.health_check_uri, "http://example.com");
    assert_eq!(service.base_uri, "http://example.com");
    assert_eq!(service.dependencies, dependencies);
    assert_eq!(service.endpoints, endpoints);
}

#[test]
fn test_count_service() {
    let conn = &mut postgres_connection();

    let result = service::Service::count(conn);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert_eq!(result.unwrap(), 0);

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::count(conn);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_check_if_service_id_exists() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::check_if_service_id_exists(conn, 1);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert!(result.unwrap());
}

#[test]
fn test_check_if_service_id_online() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    // Test if online
    let result = service::Service::check_if_service_id_online(conn, 1);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert!(result.unwrap());
}

#[test]
fn test_get_all_online_services() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::get_all_online_services(conn);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_get_all_offline_services() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::get_all_offline_services(conn);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_get_all_service_dependencies() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service_id = 1;

    let result = service::Service::get_all_service_dependencies(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_get_all_service_endpoints() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service_id = 1;

    let result = service::Service::get_all_service_endpoints(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_service_read() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service_id = 1;

    let result = service::Service::read(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service = result.unwrap();

    assert_eq!(service.service_id, 1);
    assert_eq!(service.name, "test");
    assert_eq!(service.version, 1);
    assert!(service.online);
    assert_eq!(service.description, "test");
    assert_eq!(service.health_check_uri, "http://example.com");
    assert_eq!(service.base_uri, "http://example.com");
    assert_eq!(service.dependencies, vec![Some(42)]);
}

#[test]
fn test_service_read_all() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::read_all(conn);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let services = result.unwrap();
    assert!(!services.is_empty());
}

#[test]
fn test_set_service_online() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service_id = 1;

    let result = service::Service::set_service_online(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::check_if_service_id_online(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert!(result.unwrap());
}

#[test]
fn test_set_service_offline() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service_id = 1;

    let result = service::Service::set_service_offline(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let result = service::Service::check_if_service_id_online(conn, service_id);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert!(!result.unwrap());
}

#[test]
fn test_service_update() {
    let conn = &mut postgres_connection();

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    // check if service_id exists so we can update the service
    let result = service::Service::check_if_service_id_exists(conn, 1);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert!(result.unwrap());

    let update = UpdateService::new(
        Some("new_test".to_string()),
        Some(2),
        Some(true),
        None,
        None,
        None,
        None,
        None,
    );

    let result = service::Service::update(conn, 1, &update);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    let service = result.unwrap();

    assert_eq!(service.service_id, 1);
    assert_eq!(service.name, "new_test");
    assert_eq!(service.version, 2);
    assert!(service.online);
    assert_eq!(service.description, "test");
    assert_eq!(service.health_check_uri, "http://example.com");
    assert_eq!(service.base_uri, "http://example.com");
    assert_eq!(service.dependencies.len(), 1);
    assert_eq!(service.dependencies, vec![Some(42)]);
}

#[test]
fn test_service_delete() {
    let conn = &mut postgres_connection();

    // Insert the service
    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    // Check if its there
    let result = service::Service::read(conn, 1);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    // Delete service
    let result = service::Service::delete(conn, 1);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());

    // Check its gone
    let result = service::Service::read(conn, 1);
    //dbg!(&result);
    assert!(result.is_err(), "{:?}", result.unwrap_err());

    let result = service::Service::count(conn);
    assert!(result.is_ok(), "{:?}", result.unwrap_err());
    assert_eq!(result.unwrap(), 0);
}

fn get_crate_service() -> CreateService {
    let endpoints = get_endpoints();

    let dependencies = get_dependencies();

    CreateService {
        service_id: 1,
        name: "test".to_string(),
        version: 1,
        online: true,
        description: "test".to_string(),
        health_check_uri: "http://example.com".to_string(),
        base_uri: "http://example.com".to_string(),
        dependencies,
        endpoints,
    }
}

fn get_endpoints() -> Vec<Option<Endpoint>> {
    let grpc_endpoint = Endpoint::new(
        "test_grpc_endpoint".to_string(),
        1,
        "/grpc".to_string(),
        7070,
        ProtocolType::GRPC,
    );

    let http_endpoint = Endpoint::new(
        "test_http_endpoint".to_string(),
        1,
        "/http".to_string(),
        8080,
        ProtocolType::HTTP,
    );

    vec![Some(grpc_endpoint.clone()), Some(http_endpoint.clone())]
}

fn get_dependencies() -> Vec<Option<i32>> {
    vec![Some(42)]
}
