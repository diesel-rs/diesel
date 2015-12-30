use postgresql::connection::PgConnection;
use query_builder::{QueryBuilder, Binds, BuildQueryResult};
use types::NativeSqlType;

pub struct PgQueryBuilder<'a> {
    conn: &'a PgConnection,
    pub sql: String,
    pub binds: Binds,
    pub bind_types: Vec<u32>,
    bind_idx: u32,
}

impl<'a> PgQueryBuilder<'a> {
    pub fn new(conn: &'a PgConnection) -> Self {
        PgQueryBuilder {
            conn: conn,
            sql: String::new(),
            binds: Vec::new(),
            bind_types: Vec::new(),
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

    fn push_bound_value(&mut self, tpe: &NativeSqlType, bind: Option<Vec<u8>>) {
        self.bind_idx += 1;
        let sql = format!("${}", self.bind_idx);
        self.push_sql(&sql);
        self.binds.push(bind);
        self.bind_types.push(tpe.oid());
    }
}
