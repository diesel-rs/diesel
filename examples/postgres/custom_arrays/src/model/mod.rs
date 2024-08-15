pub mod endpoint_type;
pub mod service;

pub mod protocol_type;

pub type Connection =
    diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>;
