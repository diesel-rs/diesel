use core::marker::PhantomData;
use diesel::backend::Backend;
use diesel::mysql_like::data_types::{MysqlTime, MysqlTimestampType};
use diesel::mysql_like::MysqlLikeBackend;
use diesel::mysql_like::{MysqlType, MysqlValue};
use diesel::row::{PartialRow, RowIndex, RowSealed};
use mysql_async::consts::{ColumnFlags, ColumnType};
use mysql_async::{Column, Row, Value};
use std::borrow::Cow;

pub struct MysqlLikeRow<DB: MysqlLikeBackend>(pub(super) Row, PhantomData<DB>);

impl<DB: MysqlLikeBackend> mysql_async::prelude::FromRow for MysqlLikeRow<DB> {
    fn from_row_opt(row: Row) -> Result<Self, mysql_async::FromRowError>
    where
        Self: Sized,
    {
        Ok(Self(row, PhantomData))
    }
}

impl<DB: MysqlLikeBackend> RowIndex<usize> for MysqlLikeRow<DB> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.0.columns_ref().len() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a, DB: MysqlLikeBackend> RowIndex<&'a str> for MysqlLikeRow<DB> {
    fn idx(&self, idx: &'a str) -> Option<usize> {
        self.0.columns().iter().position(|c| c.name_str() == idx)
    }
}

impl<DB: MysqlLikeBackend> RowSealed for MysqlLikeRow<DB> {}

impl<'a, DB: MysqlLikeBackend> diesel::row::Row<'a, DB> for MysqlLikeRow<DB> {
    type InnerPartialRow = Self;
    type Field<'b>
        = MysqlLikeField<'b, DB>
    where
        Self: 'b,
        'a: 'b;

    fn field_count(&self) -> usize {
        self.0.columns_ref().len()
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: diesel::row::RowIndex<I>,
    {
        let idx = diesel::row::RowIndex::idx(self, idx)?;
        let value = self.0.as_ref(idx)?;
        let column = &self.0.columns_ref()[idx];
        let buffer = match value {
            Value::NULL => None,
            Value::Bytes(b) => {
                // deserialize gets the length prepended, so we just use that buffer
                // directly
                Some(Cow::Borrowed(b as &[_]))
            }
            Value::Time(neg, day, hour, minute, second, second_part) => {
                let date = MysqlTime::new(
                    0,
                    0,
                    *day as _,
                    *hour as _,
                    *minute as _,
                    *second as _,
                    *second_part as _,
                    *neg as _,
                    MysqlTimestampType::MYSQL_TIMESTAMP_TIME,
                    0,
                );
                let buffer = date.serialize().to_vec();
                Some(Cow::Owned(buffer))
            }
            Value::Date(year, month, day, hour, minute, second, second_part) => {
                let date = MysqlTime::new(
                    *year as _,
                    *month as _,
                    *day as _,
                    *hour as _,
                    *minute as _,
                    *second as _,
                    *second_part as _,
                    false,
                    MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
                    0,
                );
                let buffer = date.serialize().to_vec();
                Some(Cow::Owned(buffer))
            }
            _t => {
                let mut buffer = Vec::with_capacity(
                    value
                        .bin_len()
                        .try_into()
                        .expect("Failed to cast byte size to usize"),
                );
                mysql_common::proto::MySerialize::serialize(value, &mut buffer);
                Some(Cow::Owned(buffer))
            }
        };
        let field = MysqlLikeField {
            value: buffer,
            column,
            name: column.name_str(),
            _phantom: PhantomData,
        };
        Some(field)
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new::<DB>(self, range)
    }
}

pub struct MysqlLikeField<'a, DB> {
    value: Option<Cow<'a, [u8]>>,
    column: &'a Column,
    name: Cow<'a, str>,
    _phantom: PhantomData<DB>,
}

impl<DB: MysqlLikeBackend> diesel::row::Field<'_, DB> for MysqlLikeField<'_, DB> {
    fn field_name(&self) -> Option<&str> {
        Some(&*self.name)
    }

    fn value(&self) -> Option<<DB as Backend>::RawValue<'_>> {
        self.value.as_ref().map(|v| {
            MysqlValue::new(
                v,
                convert_type(self.column.column_type(), self.column.flags()),
            )
        })
    }
}

fn convert_type(column_type: ColumnType, column_flags: ColumnFlags) -> MysqlType {
    match column_type {
        ColumnType::MYSQL_TYPE_NEWDECIMAL | ColumnType::MYSQL_TYPE_DECIMAL => MysqlType::Numeric,
        ColumnType::MYSQL_TYPE_TINY if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) => {
            MysqlType::UnsignedTiny
        }
        ColumnType::MYSQL_TYPE_TINY => MysqlType::Tiny,
        ColumnType::MYSQL_TYPE_YEAR | ColumnType::MYSQL_TYPE_SHORT
            if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) =>
        {
            MysqlType::UnsignedShort
        }
        ColumnType::MYSQL_TYPE_YEAR | ColumnType::MYSQL_TYPE_SHORT => MysqlType::Short,
        ColumnType::MYSQL_TYPE_INT24 | ColumnType::MYSQL_TYPE_LONG
            if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) =>
        {
            MysqlType::UnsignedLong
        }
        ColumnType::MYSQL_TYPE_INT24 | ColumnType::MYSQL_TYPE_LONG => MysqlType::Long,
        ColumnType::MYSQL_TYPE_LONGLONG if column_flags.contains(ColumnFlags::UNSIGNED_FLAG) => {
            MysqlType::UnsignedLongLong
        }
        ColumnType::MYSQL_TYPE_LONGLONG => MysqlType::LongLong,
        ColumnType::MYSQL_TYPE_FLOAT => MysqlType::Float,
        ColumnType::MYSQL_TYPE_DOUBLE => MysqlType::Double,

        ColumnType::MYSQL_TYPE_TIMESTAMP => MysqlType::Timestamp,
        ColumnType::MYSQL_TYPE_DATE => MysqlType::Date,
        ColumnType::MYSQL_TYPE_TIME => MysqlType::Time,
        ColumnType::MYSQL_TYPE_DATETIME => MysqlType::DateTime,
        ColumnType::MYSQL_TYPE_BIT => MysqlType::Bit,
        ColumnType::MYSQL_TYPE_JSON => MysqlType::String,

        ColumnType::MYSQL_TYPE_VAR_STRING
        | ColumnType::MYSQL_TYPE_STRING
        | ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB
            if column_flags.contains(ColumnFlags::ENUM_FLAG) =>
        {
            MysqlType::Enum
        }
        ColumnType::MYSQL_TYPE_VAR_STRING
        | ColumnType::MYSQL_TYPE_STRING
        | ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB
            if column_flags.contains(ColumnFlags::SET_FLAG) =>
        {
            MysqlType::Set
        }

        ColumnType::MYSQL_TYPE_VAR_STRING
        | ColumnType::MYSQL_TYPE_STRING
        | ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB
            if column_flags.contains(ColumnFlags::BINARY_FLAG) =>
        {
            MysqlType::Blob
        }

        ColumnType::MYSQL_TYPE_VAR_STRING
        | ColumnType::MYSQL_TYPE_STRING
        | ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB => MysqlType::String,

        ColumnType::MYSQL_TYPE_NULL
        | ColumnType::MYSQL_TYPE_NEWDATE
        | ColumnType::MYSQL_TYPE_VARCHAR
        | ColumnType::MYSQL_TYPE_TIMESTAMP2
        | ColumnType::MYSQL_TYPE_DATETIME2
        | ColumnType::MYSQL_TYPE_TIME2
        | ColumnType::MYSQL_TYPE_TYPED_ARRAY
        | ColumnType::MYSQL_TYPE_UNKNOWN
        | ColumnType::MYSQL_TYPE_ENUM
        | ColumnType::MYSQL_TYPE_SET
        | ColumnType::MYSQL_TYPE_VECTOR
        | ColumnType::MYSQL_TYPE_GEOMETRY => {
            unimplemented!("Hit an unsupported type: {:?}", column_type)
        }
    }
}
