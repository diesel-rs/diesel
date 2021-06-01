use super::result::PgResult;
use crate::pg::{Pg, PgValue};
use crate::row::*;
use std::rc::Rc;

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct PgRow<'a> {
    db_result: Rc<PgResult<'a>>,
    row_idx: usize,
}

impl<'a> PgRow<'a> {
    pub(crate) fn new(db_result: Rc<PgResult<'a>>, row_idx: usize) -> Self {
        PgRow { db_result, row_idx }
    }
}

impl<'a> Row<'a, Pg> for PgRow<'a> {
    type Field = PgField<'a>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.db_result.column_count()
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(PgField {
            db_result: self.db_result.clone(),
            row_idx: self.row_idx,
            col_idx: idx,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a> RowIndex<usize> for PgRow<'a> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a, 'b> RowIndex<&'a str> for PgRow<'b> {
    fn idx(&self, field_name: &'a str) -> Option<usize> {
        (0..self.field_count()).find(|idx| self.db_result.column_name(*idx) == Some(field_name))
    }
}

#[allow(missing_debug_implementations)]
pub struct PgField<'a> {
    db_result: Rc<PgResult<'a>>,
    row_idx: usize,
    col_idx: usize,
}

impl<'a> Field<Pg> for PgField<'a> {
    fn field_name(&self) -> Option<&str> {
        self.db_result.column_name(self.col_idx)
    }

    fn value<'b>(&'b self) -> Option<crate::backend::RawValue<'b, Pg>> {
        let raw = self.db_result.get(self.row_idx, self.col_idx)?;
        let type_oid = self.db_result.column_type(self.col_idx);

        Some(PgValue::new(raw, type_oid))
    }
}
