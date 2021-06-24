use std::cell::RefCell;
use std::convert::TryFrom;
use std::rc::Rc;

use super::sqlite_value::{OwnedSqliteValue, SqliteValue};
use super::stmt::StatementUse;
use crate::row::{Field, PartialRow, Row, RowIndex};
use crate::sqlite::Sqlite;
use crate::util::OnceCell;

#[allow(missing_debug_implementations)]
pub struct SqliteRow<'a> {
    pub(super) inner: Rc<RefCell<PrivateSqliteRow<'a>>>,
    pub(super) field_count: usize,
}

pub(super) enum PrivateSqliteRow<'a> {
    Direct(StatementUse<'a>),
    Duplicated {
        values: Vec<Option<OwnedSqliteValue>>,
        column_names: Rc<Vec<Option<String>>>,
    },
    TemporaryEmpty,
}

impl<'a> PrivateSqliteRow<'a> {
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
                        .map(|idx| stmt.copy_value(idx))
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

impl<'a> Row<'a, Sqlite> for SqliteRow<'a> {
    type Field = SqliteField<'a>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.field_count
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(SqliteField {
            row: SqliteRow {
                inner: self.inner.clone(),
                field_count: self.field_count,
            },
            col_idx: i32::try_from(idx).ok()?,
            field_name: OnceCell::new(),
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a> RowIndex<usize> for SqliteRow<'a> {
    #[inline(always)]
    fn idx(&self, idx: usize) -> Option<usize> {
        Some(idx)
        // if idx < self.field_count {
        //     Some(idx)
        // } else {
        //     None
        // }
    }
}

impl<'a, 'd> RowIndex<&'d str> for SqliteRow<'a> {
    fn idx(&self, field_name: &'d str) -> Option<usize> {
        match &mut *self.inner.borrow_mut() {
            PrivateSqliteRow::Direct(stmt) => stmt.index_for_column_name(field_name),
            PrivateSqliteRow::Duplicated { column_names, .. } => column_names
                .iter()
                .position(|n| n.as_ref().map(|s| s as &str) == Some(field_name)),
            PrivateSqliteRow::TemporaryEmpty => {
                // This cannot happen as this is only a temproray state
                // used inside of `StatementIterator::next()`
                unreachable!(
                    "You've reached an impossible internal state. \
                     If you ever see this error message please open \
                     an issue at https://github.com/diesel-rs/diesel \
                     providing example code how to trigger this error."
                )
            }
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct SqliteField<'a> {
    pub(super) row: SqliteRow<'a>,
    pub(super) col_idx: i32,
    field_name: OnceCell<Option<String>>,
}

impl<'a> Field<'a, Sqlite> for SqliteField<'a> {
    fn field_name(&self) -> Option<&str> {
        self.field_name
            .get_or_init(|| match &mut *self.row.inner.borrow_mut() {
                PrivateSqliteRow::Direct(stmt) => {
                    stmt.field_name(self.col_idx).map(|s| s.to_owned())
                }
                PrivateSqliteRow::Duplicated { column_names, .. } => column_names
                    .get(self.col_idx as usize)
                    .and_then(|n| n.clone()),
                PrivateSqliteRow::TemporaryEmpty => {
                    // This cannot happen as this is only a temproray state
                    // used inside of `StatementIterator::next()`
                    unreachable!(
                        "You've reached an impossible internal state. \
                         If you ever see this error message please open \
                         an issue at https://github.com/diesel-rs/diesel \
                         providing example code how to trigger this error."
                    )
                }
            })
            .as_ref()
            .map(|s| s as &str)
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value<'d>(&'d self) -> Option<crate::backend::RawValue<'d, Sqlite>>
    where
        'a: 'd,
    {
        SqliteValue::new(self.row.inner.borrow(), self.col_idx)
    }
}
