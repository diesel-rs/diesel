use crate::mariadb::Mariadb;

/// A connection to a Mariadb database. Connection URLs should be in the form
/// `mariadb://[user[:password]@]host/database_name[?unix_socket=socket-path&ssl_mode=SSL_MODE*&ssl_ca=/etc/ssl/certs/ca-certificates.crt&ssl_cert=/etc/ssl/certs/client-cert.crt&ssl_key=/etc/ssl/certs/client-key.crt]`
///
///* `host` can be an IP address or a hostname. If it is set to `localhost`, a connection
///  will be attempted through the socket at `/tmp/mariadb.sock`. If you want to connect to
///  a local server via TCP (e.g. docker containers), use `0.0.0.0` or `127.0.0.1` instead.
/// * `unix_socket` expects the path to the unix socket
/// * `ssl_ca` accepts a path to the system's certificate roots
/// * `ssl_cert` accepts a path to the client's certificate file
/// * `ssl_key` accepts a path to the client's private key file
/// * `ssl_mode` expects a value defined for MySQL client command option `--ssl-mode`
///   See <https://dev.mysql.com/doc/refman/5.7/en/connection-options.html#option_general_ssl-mode>
///
/// # Supported loading model implementations
///
/// * [`DefaultLoadingMode`]
///
/// As `MariadbConnection` only supports a single loading mode implementation
/// it is **not required** to explicitly specify a loading mode
/// when calling [`RunQueryDsl::load_iter()`] or [`LoadConnection::load`]
///
/// ## DefaultLoadingMode
///
/// `MariadbConnection` only supports a single loading mode, which loads
/// values row by row from the result set.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users;
/// #     let connection = &mut establish_connection();
/// use diesel::connection::DefaultLoadingMode;
/// { // scope to restrict the lifetime of the iterator
///     let iter1 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
///
///     for r in iter1 {
///         let (id, name) = r?;
///         println!("Id: {} Name: {}", id, name);
///     }
/// }
///
/// // works without specifying the loading mode
/// let iter2 = users::table.load_iter::<(i32, String), _>(connection)?;
///
/// for r in iter2 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
/// #   Ok(())
/// # }
/// ```
///
/// This mode does **not support** creating
/// multiple iterators using the same connection.
///
/// ```compile_fail
/// # include!("../../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users;
/// #     let connection = &mut establish_connection();
/// use diesel::connection::DefaultLoadingMode;
///
/// let iter1 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
/// let iter2 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
///
/// for r in iter1 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
///
/// for r in iter2 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
/// #   Ok(())
/// # }
/// ```
pub type MariadbConnection = crate::mysql_like::MysqlLikeConnection<Mariadb>;

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use super::*;
    use std::env;

    use crate::connection::Connection;
    use crate::connection::SimpleConnection;
    use crate::query_dsl::RunQueryDsl;

    fn connection() -> MariadbConnection {
        dotenvy::dotenv().ok();
        let database_url = env::var("MARIADB_UNIT_TEST_DATABASE_URL")
            .or_else(|_| env::var("MARIADB_DATABASE_URL"))
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run unit tests");
        MariadbConnection::establish(&database_url).unwrap()
    }

    #[diesel_test_helper::test]
    fn batch_execute_handles_single_queries_with_results() {
        let connection = &mut connection();
        assert!(connection.batch_execute("SELECT 1").is_ok());
        assert!(connection.batch_execute("SELECT 1").is_ok());
    }

    #[diesel_test_helper::test]
    fn batch_execute_handles_multi_queries_with_results() {
        let connection = &mut connection();
        let query = "SELECT 1; SELECT 2; SELECT 3;";
        assert!(connection.batch_execute(query).is_ok());
        assert!(connection.batch_execute(query).is_ok());
    }

    #[diesel_test_helper::test]
    fn execute_handles_queries_which_return_results() {
        let connection = &mut connection();
        assert!(crate::sql_query("SELECT 1").execute(connection).is_ok());
        assert!(crate::sql_query("SELECT 1").execute(connection).is_ok());
    }

    #[diesel_test_helper::test]
    fn check_client_found_rows_flag() {
        let conn = &mut crate::test_helpers::connection();
        crate::sql_query("DROP TABLE IF EXISTS update_test CASCADE")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE update_test(id INTEGER PRIMARY KEY, num INTEGER NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO update_test(id, num) VALUES (1, 5)")
            .execute(conn)
            .unwrap();

        let output = crate::sql_query("UPDATE update_test SET num = 5 WHERE id = 1")
            .execute(conn)
            .unwrap();

        assert_eq!(output, 1);
    }
}
