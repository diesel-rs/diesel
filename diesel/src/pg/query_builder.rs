use std::rc::Rc;

use super::backend::Pg;
use super::connection::raw::RawConnection;
use query_builder::{QueryBuilder, BuildQueryResult};

#[allow(missing_debug_implementations)]
pub struct PgQueryBuilder {
    conn: Rc<RawConnection>,
    pub sql: String,
    bind_idx: u32,
}

impl PgQueryBuilder {
    pub fn new(conn: &Rc<RawConnection>) -> Self {
        PgQueryBuilder {
            conn: conn.clone(),
            sql: String::new(),
            bind_idx: 0,
        }
    }
}

impl QueryBuilder<Pg> for PgQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        let escaped_identifier = try!(self.conn.escape_identifier(identifier));
        Ok(self.push_sql(&escaped_identifier))
    }

    fn push_bind_param(&mut self) {
        self.bind_idx += 1;
        let sql = format!("${}", self.bind_idx);
        self.push_sql(&sql);
    }
}
