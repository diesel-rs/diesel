use super::{Statement, Binds, ffi, libc};
use result::QueryResult;
use row::Row;
use mysql::Mysql;

pub struct StatementIterator<'a> {
    stmt: &'a mut Statement,
    output_binds: Binds,
}

impl<'a> StatementIterator<'a> {
    pub fn new(stmt: &'a mut Statement) -> QueryResult<Self> {
        use result::Error::QueryBuilderError;

        let mut result_metadata = match try!(stmt.result_metadata()) {
            Some(result) => result,
            None => return Err(QueryBuilderError("Attempted to get results \
                on a query with no results".into())),
        };
        let result_types = result_metadata.fields().map(|f| f.type_);
        let mut output_binds = Binds::from_output_types(result_types);

        unsafe {
            output_binds.with_mysql_binds(|bind_ptr| stmt.bind_result(bind_ptr))?
        }

        Ok(StatementIterator {
            stmt: stmt,
            output_binds: output_binds,
        })
    }

    pub fn map<F, T>(mut self, mut f: F) -> QueryResult<Vec<T>> where
        F: FnMut(MysqlRow) -> QueryResult<T>,
    {
        let mut results = Vec::new();
        while let Some(row) = self.next() {
            results.push(f(row?)?);
        }
        Ok(results)
    }

    fn next(&mut self) -> Option<QueryResult<MysqlRow>> {
        let next_row_result = unsafe { ffi::mysql_stmt_fetch(self.stmt.stmt) };
        match next_row_result as libc::c_uint {
            ffi::MYSQL_NO_DATA => return None,
            ffi::MYSQL_DATA_TRUNCATED => {
                let res = self.output_binds.populate_dynamic_buffers(&self.stmt);
                if let Err(e) = res {
                    return Some(Err(e));
                }
            }
            0 => self.output_binds.update_buffer_lengths(),
            _error => if let Err(e) = self.stmt.did_an_error_occur() {
                return Some(Err(e));
            }
        }

        Some(Ok(MysqlRow {
            col_idx: 0,
            binds: &mut self.output_binds,
        }))
    }
}

pub struct MysqlRow<'a> {
    col_idx: usize,
    binds: &'a Binds,
}

impl<'a> Row<Mysql> for MysqlRow<'a> {
    fn take(&mut self) -> Option<&[u8]> {
        let current_idx = self.col_idx;
        self.col_idx += 1;
        self.binds.field_data(current_idx)
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| self.binds.field_data(self.col_idx + i).is_none())
    }
}
