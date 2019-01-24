use std::collections::HashMap;

use super::{ffi, libc, Binds, Statement, StatementMetadata};
use mysql::{Mysql, MysqlType};
use result::QueryResult;
use row::*;
use sql_types::IsSigned;

pub struct StatementIterator<'a> {
    stmt: &'a mut Statement,
    output_binds: Binds,
}

#[allow(clippy::should_implement_trait)] // don't neet `Iterator` here
impl<'a> StatementIterator<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(stmt: &'a mut Statement, types: Vec<(MysqlType, IsSigned)>) -> QueryResult<Self> {
        let mut output_binds = Binds::from_output_types(types);

        execute_statement(stmt, &mut output_binds)?;

        Ok(StatementIterator {
            stmt: stmt,
            output_binds: output_binds,
        })
    }

    pub fn map<F, T>(mut self, mut f: F) -> QueryResult<Vec<T>>
    where
        F: FnMut(MysqlRow) -> QueryResult<T>,
    {
        let mut results = Vec::new();
        while let Some(row) = self.next() {
            results.push(f(row?)?);
        }
        Ok(results)
    }

    fn next(&mut self) -> Option<QueryResult<MysqlRow>> {
        match populate_row_buffers(self.stmt, &mut self.output_binds) {
            Ok(Some(())) => Some(Ok(MysqlRow {
                col_idx: 0,
                binds: &mut self.output_binds,
            })),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
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

pub struct NamedStatementIterator<'a> {
    stmt: &'a mut Statement,
    output_binds: Binds,
    metadata: StatementMetadata,
}

#[allow(clippy::should_implement_trait)] // don't need `Iterator` here
impl<'a> NamedStatementIterator<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(stmt: &'a mut Statement) -> QueryResult<Self> {
        let metadata = stmt.metadata()?;
        let mut output_binds = Binds::from_result_metadata(metadata.fields());

        execute_statement(stmt, &mut output_binds)?;

        Ok(NamedStatementIterator {
            stmt,
            output_binds,
            metadata,
        })
    }

    pub fn map<F, T>(mut self, mut f: F) -> QueryResult<Vec<T>>
    where
        F: FnMut(NamedMysqlRow) -> QueryResult<T>,
    {
        let mut results = Vec::new();
        while let Some(row) = self.next() {
            results.push(f(row?)?);
        }
        Ok(results)
    }

    fn next(&mut self) -> Option<QueryResult<NamedMysqlRow>> {
        match populate_row_buffers(self.stmt, &mut self.output_binds) {
            Ok(Some(())) => Some(Ok(NamedMysqlRow {
                binds: &self.output_binds,
                column_indices: self.metadata.column_indices(),
            })),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

pub struct NamedMysqlRow<'a> {
    binds: &'a Binds,
    column_indices: &'a HashMap<&'a str, usize>,
}

impl<'a> NamedRow<Mysql> for NamedMysqlRow<'a> {
    fn index_of(&self, column_name: &str) -> Option<usize> {
        self.column_indices.get(column_name).cloned()
    }

    fn get_raw_value(&self, idx: usize) -> Option<&[u8]> {
        self.binds.field_data(idx)
    }
}

fn execute_statement(stmt: &mut Statement, binds: &mut Binds) -> QueryResult<()> {
    unsafe {
        binds.with_mysql_binds(|bind_ptr| stmt.bind_result(bind_ptr))?;
        stmt.execute()?;
    }
    Ok(())
}

fn populate_row_buffers(stmt: &Statement, binds: &mut Binds) -> QueryResult<Option<()>> {
    let next_row_result = unsafe { ffi::mysql_stmt_fetch(stmt.stmt.as_ptr()) };
    match next_row_result as libc::c_uint {
        ffi::MYSQL_NO_DATA => Ok(None),
        ffi::MYSQL_DATA_TRUNCATED => binds.populate_dynamic_buffers(stmt).map(Some),
        0 => {
            binds.update_buffer_lengths();
            Ok(Some(()))
        }
        _error => stmt.did_an_error_occur().map(Some),
    }
}
