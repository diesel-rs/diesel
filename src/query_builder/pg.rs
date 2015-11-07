use connection::Connection;
use super::{QueryBuilder, Binds, BuildQueryResult};

pub struct PgQueryBuilder<'a> {
    conn: &'a Connection,
    pub sql: String,
    pub binds: Binds,
    bind_idx: u32,
}

impl<'a> PgQueryBuilder<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        PgQueryBuilder {
            conn: conn,
            sql: String::new(),
            binds: Vec::new(),
            bind_idx: 0,
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

    fn push_bound_value(&mut self, bind: Option<Vec<u8>>) {
        self.bind_idx += 1;
        let sql = format!("${}", self.bind_idx);
        self.push_sql(&sql);
        self.binds.push(bind);
    }
}
