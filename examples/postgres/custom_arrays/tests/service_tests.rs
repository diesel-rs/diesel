use custom_arrays::model::endpoint_type::Endpoint;
use custom_arrays::model::protocol_type::ProtocolType;
use custom_arrays::model::service;
use custom_arrays::model::service::{CreateService, UpdateService};
use custom_arrays::Connection;
use diesel::{Connection as DieselConnection, PgConnection};
use dotenvy::dotenv;
use std::env;

fn postgres_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("POSTGRES_DATABASE_URL").expect("POSTGRES_DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

fn test_db_migration(conn: &mut Connection) {
    let res = custom_arrays::run_db_migration(conn);
    //dbg!(&result);
    assert!(res.is_ok());
}

#[test]
fn test_service() {
    let mut connection = postgres_connection();
    let conn = &mut connection;

    println!("Test DB migration");
    test_db_migration(conn);

    println!("Test create!");
    test_create_service(conn);

    println!("Test count!");
    test_count_service(conn);

    println!("Test check if exists!");
    test_check_if_service_id_exists(conn);

    println!("Test check if online!");
    test_check_if_service_id_online(conn);

    println!("Test get all online services!");
    test_get_all_online_services(conn);

    println!("Test get all offline services!");
    test_get_all_offline_services(conn);

    println!("Test get all service dependencies!");
    test_get_all_service_dependencies(conn);

    println!("Test get all service endpoints!");
    test_get_all_service_endpoints(conn);

    println!("Test read!");
    test_service_read(conn);

    println!("Test read_all!");
    test_service_read_all(conn);

    println!("Test set service online!");
    test_set_service_online(conn);

    println!("Test set service offline!");
    test_set_service_offline(conn);

    println!("Test update service!");
    test_service_update(conn);

    println!("Test delete service!");
    test_service_delete(conn);
}

fn test_create_service(conn: &mut Connection) {
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

    let endpoints = vec![Some(grpc_endpoint.clone()), Some(http_endpoint.clone())];

    let dependencies = vec![Some(42)];

    let service = CreateService {
        service_id: 1,
        name: "test".to_string(),
        version: 1,
        online: true,
        description: "test".to_string(),
        health_check_uri: "http://example.com".to_string(),
        base_uri: "http://example.com".to_string(),
        dependencies,
        endpoints,
    };

    let result = service::Service::create(conn, &service);

    // dbg!(&result);
    assert!(result.is_ok());

    let service = result.unwrap();

    assert_eq!(service.service_id, 1);
    assert_eq!(service.name, "test");
    assert_eq!(service.version, 1);
    assert!(service.online);
    assert_eq!(service.description, "test");
    assert_eq!(service.health_check_uri, "http://example.com");
    assert_eq!(service.base_uri, "http://example.com");
    assert_eq!(service.dependencies, vec![Some(42)]);
    assert_eq!(
        service.endpoints,
        vec![Some(grpc_endpoint.clone()), Some(http_endpoint.clone())]
    );
}

fn test_count_service(conn: &mut Connection) {
    let result = service::Service::count(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

fn test_check_if_service_id_exists(conn: &mut Connection) {
    let result = service::Service::check_if_service_id_exists(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

fn test_check_if_service_id_online(conn: &mut Connection) {
    let result = service::Service::check_if_service_id_online(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

fn test_get_all_online_services(conn: &mut Connection) {
    let result = service::Service::get_all_online_services(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

fn test_get_all_offline_services(conn: &mut Connection) {
    let result = service::Service::get_all_offline_services(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

fn test_get_all_service_dependencies(conn: &mut Connection) {
    let service_id = 1;

    let result = service::Service::get_all_service_dependencies(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

fn test_get_all_service_endpoints(conn: &mut Connection) {
    let service_id = 1;

    let result = service::Service::get_all_service_endpoints(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

fn test_service_read(conn: &mut Connection) {
    let service_id = 1;

    let result = service::Service::read(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());

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

fn test_service_read_all(conn: &mut Connection) {
    let result = service::Service::read_all(conn);
    //dbg!(&result);
    assert!(result.is_ok());

    let services = result.unwrap();
    assert!(!services.is_empty());
}

fn test_set_service_online(conn: &mut Connection) {
    let service_id = 1;

    let result = service::Service::set_service_online(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::check_if_service_id_online(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

fn test_set_service_offline(conn: &mut Connection) {
    let service_id = 1;

    let result = service::Service::set_service_offline(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::check_if_service_id_online(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

fn test_service_update(conn: &mut Connection) {
    // check if service_id exists so we can update the service
    let result = service::Service::check_if_service_id_exists(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());
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
    //dbg!(&result);
    assert!(result.is_ok());

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

fn test_service_delete(conn: &mut Connection) {
    let result = service::Service::read(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::delete(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::read(conn, 1);
    //dbg!(&result);
    assert!(result.is_err());

    let result = service::Service::count(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}
