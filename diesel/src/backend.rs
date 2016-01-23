use query_builder::QueryBuilder;
use query_builder::pg::PgQueryBuilder;
use query_builder::debug::DebugQueryBuilder;

pub trait Backend {
    type QueryBuilder: QueryBuilder;
}

pub struct Debug;

impl Backend for Debug {
    type QueryBuilder = DebugQueryBuilder;
}

pub struct Pg;

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
}
