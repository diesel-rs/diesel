use connection::Connection;
use query_source::Queryable;
use result::QueryResult;
use types::HasSqlType;

pub trait SaveChangesDsl<Conn, ST> where
    Conn: Connection,
    Conn::Backend: HasSqlType<ST>,
{
    fn save_changes<T>(&self, connection: &Conn) -> QueryResult<T> where
        T: Queryable<ST, Conn::Backend>;
}
