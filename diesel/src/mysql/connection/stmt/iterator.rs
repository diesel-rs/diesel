use std::marker::PhantomData;
use std::rc::Rc;

use super::{Binds, Statement, StatementMetadata};
use crate::mysql::{Mysql, MysqlType};
use crate::result::QueryResult;
use crate::row::*;

pub struct StatementIterator<'a> {
    stmt: &'a mut Statement,
    output_binds: Rc<Binds>,
    metadata: Rc<StatementMetadata>,
    types: Vec<Option<MysqlType>>,
    size: usize,
    fetched_rows: usize,
}

impl<'a> StatementIterator<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(stmt: &'a mut Statement, types: Vec<Option<MysqlType>>) -> QueryResult<Self> {
        let metadata = stmt.metadata()?;

        let mut output_binds = Binds::from_output_types(&types, &metadata);

        stmt.execute_statement(&mut output_binds)?;
        let size = unsafe { stmt.result_size() }?;

        Ok(StatementIterator {
            metadata: Rc::new(metadata),
            output_binds: Rc::new(output_binds),
            fetched_rows: 0,
            size,
            stmt,
            types,
        })
    }
}

impl<'a> Iterator for StatementIterator<'a> {
    type Item = QueryResult<MysqlRow<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // check if we own the only instance of the bind buffer
        // if that's the case we can reuse the underlying allocations
        // if that's not the case, allocate a new buffer
        let res = if let Some(binds) = Rc::get_mut(&mut self.output_binds) {
            self.stmt
                .populate_row_buffers(binds)
                .map(|o| o.map(|()| self.output_binds.clone()))
        } else {
            // The shared bind buffer is in use by someone else,
            // we allocate a new buffer here
            let mut output_binds = Binds::from_output_types(&self.types, &self.metadata);
            self.stmt
                .populate_row_buffers(&mut output_binds)
                .map(|o| o.map(|()| Rc::new(output_binds)))
        };

        match res {
            Ok(Some(binds)) => {
                self.fetched_rows += 1;
                Some(Ok(MysqlRow {
                    col_idx: 0,
                    binds,
                    metadata: self.metadata.clone(),
                    _marker: Default::default(),
                }))
            }
            Ok(None) => None,
            Err(e) => {
                self.fetched_rows += 1;
                Some(Err(e))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}

impl<'a> ExactSizeIterator for StatementIterator<'a> {
    fn len(&self) -> usize {
        self.size - self.fetched_rows
    }
}

#[derive(Clone)]
pub struct MysqlRow<'a> {
    col_idx: usize,
    binds: Rc<Binds>,
    metadata: Rc<StatementMetadata>,
    _marker: PhantomData<&'a mut (Binds, StatementMetadata)>,
}

impl<'a> Row<'a, Mysql> for MysqlRow<'a> {
    type Field = MysqlField<'a>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.binds.len()
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(MysqlField {
            bind: self.binds.clone(),
            metadata: self.metadata.clone(),
            idx,
            _marker: Default::default(),
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a> RowIndex<usize> for MysqlRow<'a> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a, 'b> RowIndex<&'a str> for MysqlRow<'b> {
    fn idx(&self, idx: &'a str) -> Option<usize> {
        self.metadata
            .fields()
            .iter()
            .enumerate()
            .find(|(_, field_meta)| field_meta.field_name() == Some(idx))
            .map(|(idx, _)| idx)
    }
}

pub struct MysqlField<'a> {
    bind: Rc<Binds>,
    metadata: Rc<StatementMetadata>,
    idx: usize,
    _marker: PhantomData<&'a (Binds, StatementMetadata)>,
}

impl<'a> Field<Mysql> for MysqlField<'a> {
    fn field_name(&self) -> Option<&str> {
        self.metadata.fields()[self.idx].field_name()
    }

    fn is_null(&self) -> bool {
        (*self.bind)[self.idx].is_null()
    }

    fn value<'b>(&'b self) -> Option<crate::backend::RawValue<'b, Mysql>> {
        self.bind[self.idx].value()
    }
}
