use super::{Mysql, MysqlType};
use query_builder::BindCollector;
use result::Error::SerializationError;
use result::QueryResult;
use serialize::{IsNull, Output, ToSql};
use sql_types::{HasSqlType, IsSigned};

#[derive(Default)]
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct MysqlBindCollector {
    pub(crate) binds: Vec<(MysqlType, IsSigned, Option<Vec<u8>>)>,
}

impl MysqlBindCollector {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl BindCollector<Mysql> for MysqlBindCollector {
    fn push_bound_value<T, U>(&mut self, bind: &U, metadata_lookup: &()) -> QueryResult<()>
    where
        Mysql: HasSqlType<T>,
        U: ToSql<T, Mysql>,
    {
        let mut to_sql_output = Output::new(Vec::new(), metadata_lookup);
        let is_null = bind
            .to_sql(&mut to_sql_output)
            .map_err(SerializationError)?;
        let bytes = match is_null {
            IsNull::No => Some(to_sql_output.into_inner()),
            IsNull::Yes => None,
        };
        let metadata = Mysql::metadata(metadata_lookup);
        let sign = Mysql::is_signed();
        self.binds.push((metadata, sign, bytes));
        Ok(())
    }
}
