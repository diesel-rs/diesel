use custom_arrays::model::endpoint_type::Endpoint;
use custom_arrays::model::protocol_type::ProtocolType;
use custom_arrays::model::service;
use custom_arrays::model::service::{CreateService, UpdateService};
use custom_arrays::Connection;
use diesel::{Connection as DieselConnection, PgConnection};
use dotenvy::dotenv;
use std::env;
use std::process::Command;
use std::time::Duration;

#[allow(dead_code)]
fn start_or_reuse_postgres_docker_container() {
    // check if a container with name postgres-5432 is already running.
    // If so, re-use that container and just return.
    let output = Command::new("docker")
        .arg("ps")
        .arg(format!("--filter=name={}", "postgres-5432"))
        .arg("--format={{.Names}}")
        .output()
        .expect("failed to check for running postgres container");
    if !output.stdout.is_empty() {
        return;
    }

    // If there is no container running, start one.
    // Example: docker run --name postgres-5432 -p 5432:5432 -e POSTGRES_PASSWORD=postgres -d postgres:16.3-bookworm
    println!("start_postgres_docker_container");
    Command::new("docker")
        .args([
            "run",
            "--name",
            "postgres-5432",
            "-p",
            "5432:5432",
            "-e",
            "POSTGRES_PASSWORD=postgres",
            "-d",
            "postgres:16.3-bookworm",
        ])
        .output()
        .expect("failed to start postgres container");
    // Wait for the container to start
    std::thread::sleep(Duration::from_secs(5));
}

fn run_db_migration(conn: &mut Connection) {
    println!("run_db_migration");
    let res = custom_arrays::run_db_migration(conn);
    //dbg!(&result);
    assert!(res.is_ok());
}

fn revert_db_migration(conn: &mut Connection) {
    println!("revert_db_migration");
    let res = custom_arrays::revert_db_migration(conn);
    //dbg!(&result);
    assert!(res.is_ok());
}

fn postgres_connection() -> PgConnection {
    println!("postgres_connection");
    dotenv().ok();

    let database_url =
        env::var("POSTGRES_DATABASE_URL").expect("POSTGRES_DATABASE_URL must be set");

    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[allow(dead_code)]
fn test_teardown() {
    println!("test_teardown");

    // Optional. In some environments, you may have to tidy up everything after testing.
    let mut connection = postgres_connection();
    let conn = &mut connection;

    println!("Revert pending DB migration");
    revert_db_migration(conn);
}

// #[test] // You can test the test setup standalone i.e. for debugging.
// Run this setup first in every test.
fn test_setup() {
    println!("test_setup");

    // Optional: May start a Docker container first if you want fully self contained testing
    // If there is a container already running, the util will re-use it.
    // println!("Start or reuse postgres container");
    // start_or_reuse_postgres_docker_container();

    println!("Connect to database");
    let mut connection = postgres_connection();
    let conn = &mut connection;

    println!("Run pending DB migration");
    run_db_migration(conn);
}

#[test]
fn test_create_service() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let endpoints = get_endpoints();
    let dependencies = get_dependencies();

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
    assert_eq!(service.dependencies, dependencies);
    assert_eq!(service.endpoints, endpoints);
}

#[test]
fn test_count_service() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let result = service::Service::count(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::count(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_check_if_service_id_exists() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::check_if_service_id_exists(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_check_if_service_id_online() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    // Test if online
    let result = service::Service::check_if_service_id_online(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_get_all_online_services() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::get_all_online_services(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_get_all_offline_services() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::get_all_offline_services(conn);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_get_all_service_dependencies() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let service_id = 1;

    let result = service::Service::get_all_service_dependencies(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_get_all_service_endpoints() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let service_id = 1;

    let result = service::Service::get_all_service_endpoints(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_service_read() {
    test_setup();

    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

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

#[test]
fn test_service_read_all() {
    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::read_all(conn);
    //dbg!(&result);
    assert!(result.is_ok());

    let services = result.unwrap();
    assert!(!services.is_empty());
}

#[test]
fn test_set_service_online() {
    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let service_id = 1;

    let result = service::Service::set_service_online(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::check_if_service_id_online(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_set_service_offline() {
    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    let service_id = 1;

    let result = service::Service::set_service_offline(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());

    let result = service::Service::check_if_service_id_online(conn, service_id);
    //dbg!(&result);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_service_update() {
    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

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

#[test]
fn test_service_delete() {
    let mut connection = postgres_connection();
    let conn = &mut connection;
    conn.begin_test_transaction()
        .expect("Failed to begin test transaction");

    // Insert the service
    let service = get_crate_service();
    let result = service::Service::create(conn, &service);
    // dbg!(&result);
    assert!(result.is_ok());

    // Check if its there
    let result = service::Service::read(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());

    // Delete service
    let result = service::Service::delete(conn, 1);
    //dbg!(&result);
    assert!(result.is_ok());

    // Check its gone
    let result = service::Service::read(conn, 1);
    //dbg!(&result);
    assert!(result.is_err());

    let result = service::Service::count(conn);
    //dbg!(&result);
    assert!(result.is_ok());
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
