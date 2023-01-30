use super::result::PgResult;
use crate::backend::Backend;
use crate::pg::value::TypeOidLookup;
use crate::pg::{Pg, PgValue};
use crate::row::*;
use std::rc::Rc;

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct PgRow {
    db_result: Rc<PgResult>,
    row_idx: usize,
}

impl PgRow {
    pub(crate) fn new(db_result: Rc<PgResult>, row_idx: usize) -> Self {
        PgRow { db_result, row_idx }
    }
}

impl RowSealed for PgRow {}

impl<'a> Row<'a, Pg> for PgRow {
    type Field<'f> = PgField<'f> where 'a: 'f, Self: 'f;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.db_result.column_count()
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(PgField {
            db_result: &self.db_result,
            row_idx: self.row_idx,
            col_idx: idx,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for PgRow {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a> RowIndex<&'a str> for PgRow {
    fn idx(&self, field_name: &'a str) -> Option<usize> {
        (0..self.field_count()).find(|idx| self.db_result.column_name(*idx) == Some(field_name))
    }
}

#[allow(missing_debug_implementations)]
pub struct PgField<'a> {
    db_result: &'a PgResult,
    row_idx: usize,
    col_idx: usize,
}

impl<'a> Field<'a, Pg> for PgField<'a> {
    fn field_name(&self) -> Option<&str> {
        self.db_result.column_name(self.col_idx)
    }

    fn value(&self) -> Option<<Pg as Backend>::RawValue<'_>> {
        let raw = self.db_result.get(self.row_idx, self.col_idx)?;

        Some(PgValue::new_internal(raw, self))
    }
}

impl<'a> TypeOidLookup for PgField<'a> {
    fn lookup(&self) -> std::num::NonZeroU32 {
        self.db_result.column_type(self.col_idx)
    }
}
