use super::{metadata::MysqlFieldMetadata, BindData, Binds, Statement, StatementMetadata};
use crate::mysql::{Mysql, MysqlType};
use crate::result::QueryResult;
use crate::row::*;

pub struct StatementIterator<'a> {
    stmt: &'a mut Statement,
    output_binds: Binds,
    metadata: StatementMetadata,
}

#[allow(clippy::should_implement_trait)] // don't neet `Iterator` here
impl<'a> StatementIterator<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(stmt: &'a mut Statement, types: Vec<Option<MysqlType>>) -> QueryResult<Self> {
        let metadata = stmt.metadata()?;

        let mut output_binds = Binds::from_output_types(types, &metadata);

        stmt.execute_statement(&mut output_binds)?;

        Ok(StatementIterator {
            stmt,
            output_binds,
            metadata,
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
        match self.stmt.populate_row_buffers(&mut self.output_binds) {
            Ok(Some(())) => Some(Ok(MysqlRow {
                col_idx: 0,
                binds: &mut self.output_binds,
                metadata: &self.metadata,
            })),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[derive(Clone)]
pub struct MysqlRow<'a> {
    col_idx: usize,
    binds: &'a Binds,
    metadata: &'a StatementMetadata,
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
            bind: &self.binds[idx],
            metadata: &self.metadata.fields()[idx],
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
    bind: &'a BindData,
    metadata: &'a MysqlFieldMetadata<'a>,
}

impl<'a> Field<'a, Mysql> for MysqlField<'a> {
    fn field_name(&self) -> Option<&'a str> {
        self.metadata.field_name()
    }

    fn is_null(&self) -> bool {
        self.bind.is_null()
    }

    fn value(&self) -> Option<crate::backend::RawValue<'a, Mysql>> {
        self.bind.value()
    }
}
