use super::result::PgResult;
use crate::pg::{PgConnection, PgTypeMetadata};
use crate::result::QueryResult;

pub use super::raw::RawConnection;

pub struct Statement {
    name: Option<String>,
    param_formats: Vec<libpq::Format>,
}

impl Statement {
    pub fn execute(
        &self,
        conn: &PgConnection,
        param_data: &Vec<Option<Vec<u8>>>,
    ) -> QueryResult<PgResult> {
        conn.raw_connection.exec_prepared(
            self.name.as_deref(),
            param_data,
            &self.param_formats,
            libpq::Format::Binary,
        )
    }

    pub fn prepare(
        conn: &PgConnection,
        sql: &str,
        name: Option<&str>,
        param_types: &[PgTypeMetadata],
    ) -> QueryResult<Self> {
        let param_types_vec = param_types.iter().map(|x| x.oid).collect::<Vec<_>>();

        conn.raw_connection.prepare(name, sql, &param_types_vec)?;

        Ok(Statement {
            name: name.map(String::from),
            param_formats: vec![libpq::Format::Binary; param_types.len()],
        })
    }
}
