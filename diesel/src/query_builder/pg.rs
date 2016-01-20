use connection::Connection;
use super::{QueryBuilder, Binds, BuildQueryResult, Context};
use types::NativeSqlType;

pub struct PgQueryBuilder<'a> {
    conn: &'a Connection,
    pub sql: String,
    pub binds: Binds,
    pub bind_types: Vec<u32>,
    bind_idx: u32,
    context_stack: Vec<Context>,
}

impl<'a> PgQueryBuilder<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        PgQueryBuilder {
            conn: conn,
            sql: String::new(),
            binds: Vec::new(),
            bind_types: Vec::new(),
            bind_idx: 0,
            context_stack: Vec::new(),
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
        match (self.context_stack.first(), bind) {
            (Some(&Context::Insert), None) => self.push_sql("DEFAULT"),
            (_, bind) => {
                self.bind_idx += 1;
                let sql = format!("${}", self.bind_idx);
                self.push_sql(&sql);
                self.binds.push(bind);
                self.bind_types.push(tpe.oid());
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
