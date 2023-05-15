#![allow(unsafe_code)] // module uses ffi
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use super::{OutputBinds, Statement, StatementMetadata, StatementUse};
use crate::backend::Backend;
use crate::connection::statement_cache::MaybeCached;
use crate::mysql::{Mysql, MysqlType};
use crate::result::QueryResult;
use crate::row::*;

#[allow(missing_debug_implementations)]
pub struct StatementIterator<'a> {
    stmt: StatementUse<'a>,
    last_row: Rc<RefCell<PrivateMysqlRow>>,
    metadata: Rc<StatementMetadata>,
    len: usize,
}

impl<'a> StatementIterator<'a> {
    pub fn from_stmt(
        stmt: MaybeCached<'a, Statement>,
        types: &[Option<MysqlType>],
    ) -> QueryResult<Self> {
        let metadata = stmt.metadata()?;

        let mut output_binds = OutputBinds::from_output_types(types, &metadata);

        let mut stmt = stmt.execute_statement(&mut output_binds)?;
        let size = unsafe { stmt.result_size() }?;

        Ok(StatementIterator {
            metadata: Rc::new(metadata),
            last_row: Rc::new(RefCell::new(PrivateMysqlRow::Direct(output_binds))),
            len: size,
            stmt,
        })
    }
}

impl<'a> Iterator for StatementIterator<'a> {
    type Item = QueryResult<MysqlRow>;

    fn next(&mut self) -> Option<Self::Item> {
        // check if we own the only instance of the bind buffer
        // if that's the case we can reuse the underlying allocations
        // if that's not the case, we need to copy the output bind buffers
        // to somewhere else
        let res = if let Some(binds) = Rc::get_mut(&mut self.last_row) {
            if let PrivateMysqlRow::Direct(ref mut binds) = RefCell::get_mut(binds) {
                self.stmt.populate_row_buffers(binds)
            } else {
                // any other state than `PrivateMysqlRow::Direct` is invalid here
                // and should not happen. If this ever happens this is a logic error
                // in the code above
                unreachable!(
                    "You've reached an impossible internal state. \
                     If you ever see this error message please open \
                     an issue at https://github.com/diesel-rs/diesel \
                     providing example code how to trigger this error."
                )
            }
        } else {
            // The shared bind buffer is in use by someone else,
            // this means we copy out the values and replace the used reference
            // by the copied values. After this we can advance the statement
            // another step
            let mut last_row = {
                let mut last_row = match self.last_row.try_borrow_mut() {
                    Ok(o) => o,
                    Err(_e) => {
                        return Some(Err(crate::result::Error::DeserializationError(
                            "Failed to reborrow row. Try to release any `MysqlField` or `MysqlValue` \
                             that exists at this point"
                                .into(),
                        )));
                    }
                };
                let last_row = &mut *last_row;
                let duplicated = last_row.duplicate();
                std::mem::replace(last_row, duplicated)
            };
            let res = if let PrivateMysqlRow::Direct(ref mut binds) = last_row {
                self.stmt.populate_row_buffers(binds)
            } else {
                // any other state than `PrivateMysqlRow::Direct` is invalid here
                // and should not happen. If this ever happens this is a logic error
                // in the code above
                unreachable!(
                    "You've reached an impossible internal state. \
                     If you ever see this error message please open \
                     an issue at https://github.com/diesel-rs/diesel \
                     providing example code how to trigger this error."
                )
            };
            self.last_row = Rc::new(RefCell::new(last_row));
            res
        };

        match res {
            Ok(Some(())) => {
                self.len = self.len.saturating_sub(1);
                Some(Ok(MysqlRow {
                    metadata: self.metadata.clone(),
                    row: self.last_row.clone(),
                }))
            }
            Ok(None) => None,
            Err(e) => {
                self.len = self.len.saturating_sub(1);
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
        self.len
    }
}

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct MysqlRow {
    row: Rc<RefCell<PrivateMysqlRow>>,
    metadata: Rc<StatementMetadata>,
}

enum PrivateMysqlRow {
    Direct(OutputBinds),
    Copied(OutputBinds),
}

impl PrivateMysqlRow {
    fn duplicate(&self) -> Self {
        match self {
            Self::Copied(b) | Self::Direct(b) => Self::Copied(b.clone()),
        }
    }
}

impl RowSealed for MysqlRow {}

impl<'a> Row<'a, Mysql> for MysqlRow {
    type Field<'f> = MysqlField<'f> where 'a: 'f, Self: 'f;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.metadata.fields().len()
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(MysqlField {
            binds: self.row.borrow(),
            metadata: self.metadata.clone(),
            idx,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for MysqlRow {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a> RowIndex<&'a str> for MysqlRow {
    fn idx(&self, idx: &'a str) -> Option<usize> {
        self.metadata
            .fields()
            .iter()
            .enumerate()
            .find(|(_, field_meta)| field_meta.field_name() == Some(idx))
            .map(|(idx, _)| idx)
    }
}

#[allow(missing_debug_implementations)]
pub struct MysqlField<'a> {
    binds: Ref<'a, PrivateMysqlRow>,
    metadata: Rc<StatementMetadata>,
    idx: usize,
}

impl<'a> Field<'a, Mysql> for MysqlField<'a> {
    fn field_name(&self) -> Option<&str> {
        self.metadata.fields()[self.idx].field_name()
    }

    fn is_null(&self) -> bool {
        match &*self.binds {
            PrivateMysqlRow::Copied(b) | PrivateMysqlRow::Direct(b) => b[self.idx].is_null(),
        }
    }

    fn value(&self) -> Option<<Mysql as Backend>::RawValue<'_>> {
        match &*self.binds {
            PrivateMysqlRow::Copied(b) | PrivateMysqlRow::Direct(b) => b[self.idx].value(),
        }
    }
}

#[test]
#[allow(clippy::drop_non_drop)] // we want to explicitly extend lifetimes here
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

    crate::sql_query(
        "CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    )
    .execute(conn)
    .unwrap();
    crate::sql_query("DELETE FROM users;")
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

    {
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
    }

    {
        let collected_rows = conn.load(query).unwrap().collect::<Vec<_>>();
        assert_eq!(collected_rows.len(), 2);
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
    let first_fields = (
        Row::get(&first_row, 0).unwrap(),
        Row::get(&first_row, 1).unwrap(),
    );
    let first_values = (first_fields.0.value(), first_fields.1.value());

    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(first_values);
    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(first_fields);

    let second_row = row_iter.next().unwrap().unwrap();
    let second_fields = (
        Row::get(&second_row, 0).unwrap(),
        Row::get(&second_row, 1).unwrap(),
    );
    let second_values = (second_fields.0.value(), second_fields.1.value());

    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(second_values);
    assert!(row_iter.next().unwrap().is_err());
    std::mem::drop(second_fields);

    assert!(row_iter.next().is_none());

    let first_fields = (
        Row::get(&first_row, 0).unwrap(),
        Row::get(&first_row, 1).unwrap(),
    );
    let second_fields = (
        Row::get(&second_row, 0).unwrap(),
        Row::get(&second_row, 1).unwrap(),
    );

    let first_values = (first_fields.0.value(), first_fields.1.value());
    let second_values = (second_fields.0.value(), second_fields.1.value());

    assert_eq!(
        <i32 as FromSql<sql_types::Integer, Mysql>>::from_nullable_sql(first_values.0).unwrap(),
        expected[0].0
    );
    assert_eq!(
        <String as FromSql<sql_types::Text, Mysql>>::from_nullable_sql(first_values.1).unwrap(),
        expected[0].1
    );

    assert_eq!(
        <i32 as FromSql<sql_types::Integer, Mysql>>::from_nullable_sql(second_values.0).unwrap(),
        expected[1].0
    );
    assert_eq!(
        <String as FromSql<sql_types::Text, Mysql>>::from_nullable_sql(second_values.1).unwrap(),
        expected[1].1
    );

    let first_fields = (
        Row::get(&first_row, 0).unwrap(),
        Row::get(&first_row, 1).unwrap(),
    );
    let first_values = (first_fields.0.value(), first_fields.1.value());

    assert_eq!(
        <i32 as FromSql<sql_types::Integer, Mysql>>::from_nullable_sql(first_values.0).unwrap(),
        expected[0].0
    );
    assert_eq!(
        <String as FromSql<sql_types::Text, Mysql>>::from_nullable_sql(first_values.1).unwrap(),
        expected[0].1
    );
}
