use connection::Connection;
use super::{QueryBuilder, Binds, BuildQueryResult};

pub struct PgQueryBuilder<'a> {
    conn: &'a Connection,
    pub sql: String,
    pub binds: Binds,
}

impl<'a> PgQueryBuilder<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        PgQueryBuilder {
            conn: conn,
            sql: String::new(),
            binds: Vec::new(),
        }
    }
}

impl<'a> QueryBuilder for PgQueryBuilder<'a> {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        let escaped_identifier = try!(self.conn.escape_identifier(identifier));
        Ok(self.push_sql(&escaped_identifier))
    }

    fn push_binds(&mut self, binds: &mut Binds) {
        self.binds.append(binds);
    }
}
