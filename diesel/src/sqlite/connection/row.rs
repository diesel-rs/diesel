use std::cell::RefCell;
use std::convert::TryFrom;
use std::rc::Rc;

use super::sqlite_value::{OwnedSqliteValue, SqliteValue};
use super::stmt::StatementUse;
use crate::row::{Field, PartialRow, Row, RowIndex};
use crate::sqlite::Sqlite;

#[allow(missing_debug_implementations)]
pub struct SqliteRow<'a, 'b> {
    pub(super) inner: Rc<RefCell<PrivateSqliteRow<'a, 'b>>>,
}

pub(super) enum PrivateSqliteRow<'a, 'b> {
    Direct(StatementUse<'a, 'b>),
    Duplicated {
        values: Vec<Option<OwnedSqliteValue>>,
        column_names: Rc<Vec<Option<String>>>,
    },
    TemporaryEmpty,
}

impl<'a, 'b> PrivateSqliteRow<'a, 'b> {
    pub(super) fn duplicate(&mut self, column_names: &mut Option<Rc<Vec<Option<String>>>>) -> Self {
        match self {
            PrivateSqliteRow::Direct(stmt) => {
                let column_names = if let Some(column_names) = column_names {
                    column_names.clone()
                } else {
                    let c = Rc::new(
                        (0..stmt.column_count())
                            .map(|idx| stmt.field_name(idx).map(|s| s.to_owned()))
                            .collect::<Vec<_>>(),
                    );
                    *column_names = Some(c.clone());
                    c
                };
                PrivateSqliteRow::Duplicated {
                    values: (0..stmt.column_count())
                        .map(|idx| stmt.value(idx).map(|v| v.duplicate()))
                        .collect(),
                    column_names,
                }
            }
            PrivateSqliteRow::Duplicated {
                values,
                column_names,
            } => PrivateSqliteRow::Duplicated {
                values: values
                    .iter()
                    .map(|v| v.as_ref().map(|v| v.duplicate()))
                    .collect(),
                column_names: column_names.clone(),
            },
            PrivateSqliteRow::TemporaryEmpty => PrivateSqliteRow::TemporaryEmpty,
        }
    }
}

impl<'a, 'b> Row<'b, Sqlite> for SqliteRow<'a, 'b> {
    type Field = SqliteField<'a, 'b>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        match &*self.inner.borrow() {
            PrivateSqliteRow::Direct(stmt) => stmt.column_count() as usize,
            PrivateSqliteRow::Duplicated { values, .. } => values.len(),
            PrivateSqliteRow::TemporaryEmpty => unreachable!(),
        }
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(SqliteField {
            row: SqliteRow {
                inner: self.inner.clone(),
            },
            col_idx: i32::try_from(idx).ok()?,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a: 'b, 'b> RowIndex<usize> for SqliteRow<'a, 'b> {
    fn idx(&self, idx: usize) -> Option<usize> {
        match &*self.inner.borrow() {
            PrivateSqliteRow::Duplicated { .. } | PrivateSqliteRow::Direct(_)
                if idx < self.field_count() =>
            {
                Some(idx)
            }
            PrivateSqliteRow::Direct(_) | PrivateSqliteRow::Duplicated { .. } => None,
            PrivateSqliteRow::TemporaryEmpty => unreachable!(),
        }
    }
}

impl<'a: 'b, 'b, 'd> RowIndex<&'d str> for SqliteRow<'a, 'b> {
    fn idx(&self, field_name: &'d str) -> Option<usize> {
        match &mut *self.inner.borrow_mut() {
            PrivateSqliteRow::Direct(stmt) => stmt.index_for_column_name(field_name),
            PrivateSqliteRow::Duplicated { column_names, .. } => column_names
                .iter()
                .position(|n| n.as_ref().map(|s| s as &str) == Some(field_name)),
            PrivateSqliteRow::TemporaryEmpty => {
                unreachable!()
            }
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct SqliteField<'a, 'b> {
    row: SqliteRow<'a, 'b>,
    col_idx: i32,
}

impl<'a: 'b, 'b> Field<Sqlite> for SqliteField<'a, 'b> {
    fn field_name(&self) -> Option<&str> {
        todo!()
        //        self.stmt.field_name(self.col_idx)
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value<'d>(&'d self) -> Option<crate::backend::RawValue<'d, Sqlite>> {
        match &*self.row.inner.borrow() {
            PrivateSqliteRow::Direct(stmt) => stmt.value(self.col_idx),
            PrivateSqliteRow::Duplicated { values, .. } => {
                values.get(self.col_idx as usize).and_then(|v| {
                    v.as_ref()
                        .and_then(|v| unsafe { SqliteValue::new(v.value.as_ptr()) })
                })
            }
            PrivateSqliteRow::TemporaryEmpty => unreachable!(),
        }
    }
}
