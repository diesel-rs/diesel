use diesel::backend::Backend;
use diesel::row::{Field, PartialRow, RowIndex, RowSealed};
use std::{error::Error, num::NonZeroU32};
use tokio_postgres::{types::Type, Row};

pub struct PgRow {
    row: Row,
}

impl PgRow {
    pub(super) fn new(row: Row) -> Self {
        Self { row }
    }
}
impl RowSealed for PgRow {}

impl<'a> diesel::row::Row<'a, diesel::pg::Pg> for PgRow {
    type InnerPartialRow = Self;
    type Field<'b>
        = PgField<'b>
    where
        Self: 'b,
        'a: 'b;

    fn field_count(&self) -> usize {
        self.row.len()
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: diesel::row::RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(PgField {
            row: &self.row,
            idx,
        })
    }

    fn partial_row(
        &self,
        range: std::ops::Range<usize>,
    ) -> diesel::row::PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for PgRow {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.row.len() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a> RowIndex<&'a str> for PgRow {
    fn idx(&self, idx: &'a str) -> Option<usize> {
        self.row.columns().iter().position(|c| c.name() == idx)
    }
}

pub struct PgField<'a> {
    row: &'a Row,
    idx: usize,
}

impl<'a> Field<'a, diesel::pg::Pg> for PgField<'a> {
    fn field_name(&self) -> Option<&str> {
        Some(self.row.columns()[self.idx].name())
    }

    fn value(&self) -> Option<<diesel::pg::Pg as Backend>::RawValue<'_>> {
        let DieselFromSqlWrapper(value) = self.row.get(self.idx);
        value
    }
}

#[repr(transparent)]
struct TyWrapper(Type);

impl diesel::pg::TypeOidLookup for TyWrapper {
    fn lookup(&self) -> NonZeroU32 {
        NonZeroU32::new(self.0.oid()).unwrap()
    }
}

struct DieselFromSqlWrapper<'a>(Option<diesel::pg::PgValue<'a>>);

impl<'a> tokio_postgres::types::FromSql<'a> for DieselFromSqlWrapper<'a> {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + 'static + Send + Sync>> {
        let ty = unsafe { &*(ty as *const Type as *const TyWrapper) };
        Ok(DieselFromSqlWrapper(Some(diesel::pg::PgValue::new(
            raw, ty,
        ))))
    }

    fn accepts(ty: &Type) -> bool {
        ty.oid() != 0
    }

    fn from_sql_null(_ty: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(DieselFromSqlWrapper(None))
    }
}
