use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use super::owned_row::OwnedSqliteRow;
use super::sqlite_value::{OwnedSqliteValue, SqliteValue};
use super::stmt::StatementUse;
use crate::backend::Backend;
use crate::row::{Field, IntoOwnedRow, PartialRow, Row, RowIndex, RowSealed};
use crate::sqlite::Sqlite;

#[allow(missing_debug_implementations)]
pub struct SqliteRow<'stmt, 'query> {
    pub(super) inner: Rc<RefCell<PrivateSqliteRow<'stmt, 'query>>>,
    pub(super) field_count: usize,
}

pub(super) enum PrivateSqliteRow<'stmt, 'query> {
    Direct(StatementUse<'stmt, 'query>),
    Duplicated {
        values: Vec<Option<OwnedSqliteValue>>,
        column_names: Rc<[Option<String>]>,
    },
}

impl<'stmt> IntoOwnedRow<'stmt, Sqlite> for SqliteRow<'stmt, '_> {
    type OwnedRow = OwnedSqliteRow;

    type Cache = Option<Arc<[Option<String>]>>;

    fn into_owned(self, column_name_cache: &mut Self::Cache) -> Self::OwnedRow {
        self.inner.borrow().moveable(column_name_cache)
    }
}

impl<'stmt, 'query> PrivateSqliteRow<'stmt, 'query> {
    pub(super) fn duplicate(
        &mut self,
        column_names: &mut Option<Rc<[Option<String>]>>,
    ) -> PrivateSqliteRow<'stmt, 'query> {
        match self {
            PrivateSqliteRow::Direct(stmt) => {
                let column_names = if let Some(column_names) = column_names {
                    column_names.clone()
                } else {
                    let c: Rc<[Option<String>]> = Rc::from(
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
        }
    }

    pub(super) fn moveable(
        &self,
        column_name_cache: &mut Option<Arc<[Option<String>]>>,
    ) -> OwnedSqliteRow {
        match self {
            PrivateSqliteRow::Direct(stmt) => {
                if column_name_cache.is_none() {
                    *column_name_cache = Some(
                        (0..stmt.column_count())
                            .map(|idx| stmt.field_name(idx).map(|s| s.to_owned()))
                            .collect::<Vec<_>>()
                            .into(),
                    );
                }
                let column_names = Arc::clone(
                    column_name_cache
                        .as_ref()
                        .expect("This is initialized above"),
                );
                OwnedSqliteRow::new(
                    (0..stmt.column_count())
                        .map(|idx| stmt.copy_value(idx))
                        .collect(),
                    column_names,
                )
            }
            PrivateSqliteRow::Duplicated {
                values,
                column_names,
            } => {
                if column_name_cache.is_none() {
                    *column_name_cache = Some(
                        (*column_names)
                            .iter()
                            .map(|s| s.to_owned())
                            .collect::<Vec<_>>()
                            .into(),
                    );
                }
                let column_names = Arc::clone(
                    column_name_cache
                        .as_ref()
                        .expect("This is initialized above"),
                );
                OwnedSqliteRow::new(
                    values
                        .iter()
                        .map(|v| v.as_ref().map(|v| v.duplicate()))
                        .collect(),
                    column_names,
                )
            }
        }
    }
}

impl<'stmt, 'query> RowSealed for SqliteRow<'stmt, 'query> {}

impl<'stmt, 'query> Row<'stmt, Sqlite> for SqliteRow<'stmt, 'query> {
    type Field<'field> = SqliteField<'field, 'field> where 'stmt: 'field, Self: 'field;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.field_count
    }

    fn get<'field, I>(&'field self, idx: I) -> Option<Self::Field<'field>>
    where
        'stmt: 'field,
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(SqliteField {
            row: self.inner.borrow(),
            col_idx: i32::try_from(idx).ok()?,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'stmt, 'query> RowIndex<usize> for SqliteRow<'stmt, 'query> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'stmt, 'idx, 'query> RowIndex<&'idx str> for SqliteRow<'stmt, 'query> {
    fn idx(&self, field_name: &'idx str) -> Option<usize> {
        match &mut *self.inner.borrow_mut() {
            PrivateSqliteRow::Direct(stmt) => stmt.index_for_column_name(field_name),
            PrivateSqliteRow::Duplicated { column_names, .. } => column_names
                .iter()
                .position(|n| n.as_ref().map(|s| s as &str) == Some(field_name)),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct SqliteField<'stmt, 'query> {
    pub(super) row: Ref<'stmt, PrivateSqliteRow<'stmt, 'query>>,
    pub(super) col_idx: i32,
}

impl<'stmt, 'query> Field<'stmt, Sqlite> for SqliteField<'stmt, 'query> {
    fn field_name(&self) -> Option<&str> {
        match &*self.row {
            PrivateSqliteRow::Direct(stmt) => stmt.field_name(self.col_idx),
            PrivateSqliteRow::Duplicated { column_names, .. } => column_names
                .get(self.col_idx as usize)
                .and_then(|t| t.as_ref().map(|n| n as &str)),
        }
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value(&self) -> Option<<Sqlite as Backend>::RawValue<'_>> {
        SqliteValue::new(Ref::clone(&self.row), self.col_idx)
    }
}

#[test]
fn fun_with_row_iters() {
    crate::table! {
        #[allow(unused_parens)]
        users(id) {
            id -> Integer,
            name -> Text,
        }
    }

    use crate::connection::LoadConnection;
    use crate::deserialize::{FromSql, FromSqlRow};
    use crate::prelude::*;
    use crate::row::{Field, Row};
    use crate::sql_types;

    let conn = &mut crate::test_helpers::connection();

    crate::sql_query("CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);")
        .execute(conn)
        .unwrap();

    crate::insert_into(users::table)
        .values(vec![
            (users::id.eq(1), users::name.eq("Sean")),
            (users::id.eq(2), users::name.eq("Tess")),
        ])
        .execute(conn)
        .unwrap();

    let query = users::table.select((users::id, users::name));

    let expected = vec![(1, String::from("Sean")), (2, String::from("Tess"))];

    let row_iter = conn.load(query).unwrap();
    for (row, expected) in row_iter.zip(&expected) {
        let row = row.unwrap();

        let deserialized = <(i32, String) as FromSqlRow<
            (sql_types::Integer, sql_types::Text),
            _,
        >>::build_from_row(&row)
        .unwrap();

        assert_eq!(&deserialized, expected);
    }

    {
        let collected_rows = conn.load(query).unwrap().collect::<Vec<_>>();

        for (row, expected) in collected_rows.iter().zip(&expected) {
            let deserialized = row
                .as_ref()
                .map(|row| {
                    <(i32, String) as FromSqlRow<
                            (sql_types::Integer, sql_types::Text),
                        _,
                        >>::build_from_row(row).unwrap()
                })
                .unwrap();

            assert_eq!(&deserialized, expected);
        }
    }

    let mut row_iter = conn.load(query).unwrap();

    let first_row = row_iter.next().unwrap().unwrap();
    let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
    let first_values = (first_fields.0.value(), first_fields.1.value());

    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(first_values);
    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(first_fields);

    let second_row = row_iter.next().unwrap().unwrap();
    let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());
    let second_values = (second_fields.0.value(), second_fields.1.value());

    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(second_values);
    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(second_fields);

    assert!(row_iter.next().is_none());

    let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
    let second_fields = (second_row.get(0).unwrap(), second_row.get(1).unwrap());

    let first_values = (first_fields.0.value(), first_fields.1.value());
    let second_values = (second_fields.0.value(), second_fields.1.value());

    assert_eq!(
        <i32 as FromSql<sql_types::Integer, Sqlite>>::from_nullable_sql(first_values.0).unwrap(),
        expected[0].0
    );
    assert_eq!(
        <String as FromSql<sql_types::Text, Sqlite>>::from_nullable_sql(first_values.1).unwrap(),
        expected[0].1
    );

    assert_eq!(
        <i32 as FromSql<sql_types::Integer, Sqlite>>::from_nullable_sql(second_values.0).unwrap(),
        expected[1].0
    );
    assert_eq!(
        <String as FromSql<sql_types::Text, Sqlite>>::from_nullable_sql(second_values.1).unwrap(),
        expected[1].1
    );

    let first_fields = (first_row.get(0).unwrap(), first_row.get(1).unwrap());
    let first_values = (first_fields.0.value(), first_fields.1.value());

    assert_eq!(
        <i32 as FromSql<sql_types::Integer, Sqlite>>::from_nullable_sql(first_values.0).unwrap(),
        expected[0].0
    );
    assert_eq!(
        <String as FromSql<sql_types::Text, Sqlite>>::from_nullable_sql(first_values.1).unwrap(),
        expected[0].1
    );
}
