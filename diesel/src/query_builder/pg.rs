use std::rc::Rc;

use backend::Pg;
use connection::pg::raw::RawConnection;
use super::{QueryBuilder, Binds, BuildQueryResult, Context};
use types::HasSqlType;

pub struct PgQueryBuilder {
    conn: Rc<RawConnection>,
    pub sql: String,
    pub binds: Binds,
    pub bind_types: Vec<u32>,
    bind_idx: u32,
    context_stack: Vec<Context>,
}

impl PgQueryBuilder {
    pub fn new(conn: &Rc<RawConnection>) -> Self {
        PgQueryBuilder {
            conn: conn.clone(),
            sql: String::new(),
            binds: Vec::new(),
            bind_types: Vec::new(),
            bind_idx: 0,
            context_stack: Vec::new(),
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

    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where
        Pg: HasSqlType<T>,
    {
        match (self.context_stack.first(), bind) {
            (Some(&Context::Insert), None) => self.push_sql("DEFAULT"),
            (_, bind) => {
                self.bind_idx += 1;
                let sql = format!("${}", self.bind_idx);
                self.push_sql(&sql);
                self.binds.push(bind);
                self.bind_types.push(Pg::metadata().oid);
            }
        }
    }

    fn push_context(&mut self, context: Context) {
        self.context_stack.push(context);
    }

    fn pop_context(&mut self) {
        self.context_stack.pop();
    }
}
