//! Provides types and functions related to working with MariaDB
//!
//! Much of this module is re-exported from database agnostic locations.
//! However, if you are writing code specifically to extend Diesel on
//! MariaDB, you may need to work with this module directly.
pub(crate) mod backend;

pub(crate) mod query_builder;

pub use self::backend::{Mariadb, MariadbType};
#[cfg(feature = "mariadb")]
/// A connection to a Mariadb database. Connection URLs should be in the form
/// `mariadb://[user[:password]@]host/database_name[?unix_socket=socket-path&ssl_mode=SSL_MODE*&ssl_ca=/etc/ssl/certs/ca-certificates.crt&ssl_cert=/etc/ssl/certs/client-cert.crt&ssl_key=/etc/ssl/certs/client-key.crt]`
///
///* `host` can be an IP address or a hostname. If it is set to `localhost`, a connection
///   will be attempted through the socket at `/tmp/mariadb.sock`. If you want to connect to
///   a local server via TCP (e.g. docker containers), use `0.0.0.0` or `127.0.0.1` instead.
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
pub type MariadbConnection = crate::mysql::MysqlLikeConnection<Mariadb>;
pub use self::query_builder::MariadbQueryBuilder;

/// Raw mariadb value as received from the database
pub type MariadbValue<'a> = crate::mysql::MysqlValue<'a>;
pub use crate::mysql::NumericRepresentation;

pub use crate::mysql::data_types;

pub use crate::mysql::sql_types;
