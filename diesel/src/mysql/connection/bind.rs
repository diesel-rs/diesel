extern crate mysqlclient_sys as ffi;

use std::mem;
use std::os::raw as libc;

use super::stmt::Statement;
use crate::mysql::{MysqlType, MysqlValue};
use crate::result::QueryResult;

#[derive(Debug)]
pub struct Binds {
    data: Vec<BindData>,
}

impl Binds {
    pub fn from_input_data<Iter>(input: Iter) -> Self
    where
        Iter: IntoIterator<Item = (MysqlType, Option<Vec<u8>>)>,
    {
        let data = input
            .into_iter()
            .map(|(metadata, bytes)| BindData::for_input(metadata, bytes))
            .collect();

        Binds { data }
    }

    pub fn from_output_types(types: Vec<MysqlType>) -> Self {
        let data = types
            .into_iter()
            .map(|metadata| metadata.into())
            .map(BindData::for_output)
            .collect();

        Binds { data }
    }

    pub fn from_result_metadata(fields: &[ffi::MYSQL_FIELD]) -> Self {
        let data = fields
            .iter()
            .map(|field| {
                (
                    field.type_,
                    Flags::from_bits(field.flags).expect("No unknown flags"),
                )
            })
            .map(BindData::for_output)
            .collect();

        Binds { data }
    }

    pub fn with_mysql_binds<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(*mut ffi::MYSQL_BIND) -> T,
    {
        let mut binds = self
            .data
            .iter_mut()
            .map(|x| unsafe { x.mysql_bind() })
            .collect::<Vec<_>>();
        f(binds.as_mut_ptr())
    }

    pub fn populate_dynamic_buffers(&mut self, stmt: &Statement) -> QueryResult<()> {
        for (i, data) in self.data.iter_mut().enumerate() {
            data.did_numeric_overflow_occur()?;
            // This is safe because we are re-binding the invalidated buffers
            // at the end of this function
            unsafe {
                if let Some((mut bind, offset)) = data.bind_for_truncated_data() {
                    stmt.fetch_column(&mut bind, i, offset)?
                } else {
                    data.update_buffer_length()
                }
            }
        }

        unsafe { self.with_mysql_binds(|bind_ptr| stmt.bind_result(bind_ptr)) }
    }

    pub fn update_buffer_lengths(&mut self) {
        for data in &mut self.data {
            data.update_buffer_length();
        }
    }

    pub fn field_data(&self, idx: usize) -> Option<MysqlValue<'_>> {
        let data = &self.data[idx];
        self.data[idx].bytes().map(|bytes| {
            let tpe = (data.tpe, data.flags).into();
            MysqlValue::new(bytes, tpe)
        })
    }
}

bitflags::bitflags! {
    struct Flags: u32 {
        const NOT_NULL_FLAG = 1;
        const PRI_KEY_FAG = 2;
        const UNIQUE_KEY_FLAG = 4;
        const MULTIPLE_KEY_FLAG = 8;
        const BLOB_FLAG = 16;
        const UNSIGNED_FLAG = 32;
        const ZEROFILL_FLAG = 64;
        const BINARY_FLAG = 128;
        const ENUM_FLAG = 256;
        const AUTO_INCREMENT_FLAG = 512;
        const TIMESTAMP_FLAG = 1024;
        const SET_FLAG = 2048;
        const NO_DEFAULT_VALUE_FLAG = 4096;
        const ON_UPDATE_NOW_FLAG = 8192;
        const NUM_FLAG = 32768;
        const PART_KEY_FLAG = 16384;
        const GROUP_FLAG = 32768;
        const UNIQUE_FLAG = 65536;
        const BINCMP_FLAG = 130172;
        const GET_FIXED_FIELDS_FLAG = (1<<18);
        const FIELD_IN_PART_FUNC_FLAG = (1 << 19);
    }
}

#[derive(Debug)]
struct BindData {
    tpe: ffi::enum_field_types,
    bytes: Vec<u8>,
    length: libc::c_ulong,
    flags: Flags,
    is_null: ffi::my_bool,
    is_truncated: Option<ffi::my_bool>,
}

impl BindData {
    fn for_input(tpe: MysqlType, data: Option<Vec<u8>>) -> Self {
        let is_null = if data.is_none() { 1 } else { 0 };
        let bytes = data.unwrap_or_default();
        let length = bytes.len() as libc::c_ulong;
        let (tpe, flags) = tpe.into();
        BindData {
            tpe,
            bytes,
            length,
            is_null,
            is_truncated: None,
            flags,
        }
    }

    fn for_output((tpe, flags): (ffi::enum_field_types, Flags)) -> Self {
        let bytes = known_buffer_size_for_ffi_type(tpe)
            .map(|len| vec![0; len])
            .unwrap_or_default();
        let length = bytes.len() as libc::c_ulong;

        BindData {
            tpe,
            bytes,
            length,
            is_null: 0,
            is_truncated: Some(0),
            flags,
        }
    }

    fn is_truncated(&self) -> bool {
        self.is_truncated.unwrap_or(0) != 0
    }

    fn is_fixed_size_buffer(&self) -> bool {
        known_buffer_size_for_ffi_type(self.tpe).is_some()
    }

    fn bytes(&self) -> Option<&[u8]> {
        if self.is_null == 0 {
            Some(&*self.bytes)
        } else {
            None
        }
    }

    fn update_buffer_length(&mut self) {
        use std::cmp::min;

        let actual_bytes_in_buffer = min(self.bytes.capacity(), self.length as usize);
        unsafe { self.bytes.set_len(actual_bytes_in_buffer) }
    }

    unsafe fn mysql_bind(&mut self) -> ffi::MYSQL_BIND {
        let mut bind: ffi::MYSQL_BIND = mem::zeroed();
        bind.buffer_type = self.tpe;
        bind.buffer = self.bytes.as_mut_ptr() as *mut libc::c_void;
        bind.buffer_length = self.bytes.capacity() as libc::c_ulong;
        bind.length = &mut self.length;
        bind.is_null = &mut self.is_null;
        bind.is_unsigned = self.flags.contains(Flags::UNSIGNED_FLAG) as ffi::my_bool;

        if let Some(ref mut is_truncated) = self.is_truncated {
            bind.error = is_truncated;
        }

        bind
    }

    /// Resizes the byte buffer to fit the value of `self.length`, and returns
    /// a tuple of a bind pointing at the truncated data, and the offset to use
    /// in order to read the truncated data into it.
    ///
    /// This invalidates the bind previously returned by `mysql_bind`. Calling
    /// this function is unsafe unless the binds are immediately rebound.
    unsafe fn bind_for_truncated_data(&mut self) -> Option<(ffi::MYSQL_BIND, usize)> {
        if self.is_truncated() {
            let offset = self.bytes.capacity();
            let truncated_amount = self.length as usize - offset;

            debug_assert!(
                truncated_amount > 0,
                "output buffers were invalidated \
                 without calling `mysql_stmt_bind_result`"
            );
            self.bytes.set_len(offset);
            self.bytes.reserve(truncated_amount);
            self.bytes.set_len(self.length as usize);

            let mut bind = self.mysql_bind();
            bind.buffer = self.bytes[offset..].as_mut_ptr() as *mut libc::c_void;
            bind.buffer_length = truncated_amount as libc::c_ulong;
            Some((bind, offset))
        } else {
            None
        }
    }

    fn did_numeric_overflow_occur(&self) -> QueryResult<()> {
        use crate::result::Error::DeserializationError;

        if self.is_truncated() && self.is_fixed_size_buffer() {
            Err(DeserializationError(
                "Numeric overflow/underflow occurred".into(),
            ))
        } else {
            Ok(())
        }
    }
}

impl From<MysqlType> for (ffi::enum_field_types, Flags) {
    fn from(tpe: MysqlType) -> Self {
        use self::ffi::enum_field_types::*;
        let mut flags = Flags::empty();
        let tpe = match tpe {
            MysqlType::Tiny => MYSQL_TYPE_TINY,
            MysqlType::Short => MYSQL_TYPE_SHORT,
            MysqlType::Long => MYSQL_TYPE_LONG,
            MysqlType::LongLong => MYSQL_TYPE_LONGLONG,
            MysqlType::Float => MYSQL_TYPE_FLOAT,
            MysqlType::Double => MYSQL_TYPE_DOUBLE,
            MysqlType::Time => MYSQL_TYPE_TIME,
            MysqlType::Date => MYSQL_TYPE_DATE,
            MysqlType::DateTime => MYSQL_TYPE_DATETIME,
            MysqlType::Timestamp => MYSQL_TYPE_TIMESTAMP,
            MysqlType::String => MYSQL_TYPE_STRING,
            MysqlType::Blob => MYSQL_TYPE_BLOB,
            MysqlType::Numeric => MYSQL_TYPE_NEWDECIMAL,
            MysqlType::Bit => MYSQL_TYPE_BIT,
            MysqlType::Json => MYSQL_TYPE_JSON,
            MysqlType::UnsignedTiny => {
                flags = Flags::UNSIGNED_FLAG;
                MYSQL_TYPE_TINY
            }
            MysqlType::UnsignedShort => {
                flags = Flags::UNSIGNED_FLAG;
                MYSQL_TYPE_SHORT
            }
            MysqlType::UnsignedLong => {
                flags = Flags::UNSIGNED_FLAG;
                MYSQL_TYPE_LONG
            }
            MysqlType::UnsignedLongLong => {
                flags = Flags::UNSIGNED_FLAG;
                MYSQL_TYPE_LONGLONG
            }
            MysqlType::Set => {
                flags = Flags::SET_FLAG;
                MYSQL_TYPE_STRING
            }
            MysqlType::Enum => {
                flags = Flags::ENUM_FLAG;
                MYSQL_TYPE_STRING
            }
        };
        (tpe, flags)
    }
}

impl From<(ffi::enum_field_types, Flags)> for MysqlType {
    fn from((tpe, flags): (ffi::enum_field_types, Flags)) -> Self {
        use self::ffi::enum_field_types::*;

        let is_unsigned = flags.contains(Flags::UNSIGNED_FLAG);

        // https://docs.oracle.com/cd/E17952_01/mysql-8.0-en/c-api-data-structures.html
        // https://dev.mysql.com/doc/dev/mysql-server/8.0.12/binary__log__types_8h.html
        // https://dev.mysql.com/doc/internals/en/binary-protocol-value.html
        // https://mariadb.com/kb/en/packet_bindata/
        match tpe {
            MYSQL_TYPE_TINY if is_unsigned => MysqlType::UnsignedTiny,
            MYSQL_TYPE_YEAR | MYSQL_TYPE_SHORT if is_unsigned => MysqlType::UnsignedShort,
            MYSQL_TYPE_INT24 | MYSQL_TYPE_LONG if is_unsigned => MysqlType::UnsignedLong,
            MYSQL_TYPE_LONGLONG if is_unsigned => MysqlType::UnsignedLongLong,
            MYSQL_TYPE_TINY => MysqlType::Tiny,
            MYSQL_TYPE_SHORT => MysqlType::Short,
            MYSQL_TYPE_INT24 | MYSQL_TYPE_LONG => MysqlType::Long,
            MYSQL_TYPE_LONGLONG => MysqlType::LongLong,
            MYSQL_TYPE_FLOAT => MysqlType::Float,
            MYSQL_TYPE_DOUBLE => MysqlType::Double,
            MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL => MysqlType::Numeric,
            MYSQL_TYPE_BIT => MysqlType::Bit,

            MYSQL_TYPE_TIME => MysqlType::Time,
            MYSQL_TYPE_DATE => MysqlType::Date,
            MYSQL_TYPE_DATETIME => MysqlType::DateTime,
            MYSQL_TYPE_TIMESTAMP => MysqlType::Timestamp,
            MYSQL_TYPE_JSON => MysqlType::Json,

            // The documentation states that
            // MYSQL_TYPE_STRING is used for enums and sets
            // but experimentation has shown that
            // just any string like type works, so
            // better be safe here
            MYSQL_TYPE_BLOB
            | MYSQL_TYPE_TINY_BLOB
            | MYSQL_TYPE_MEDIUM_BLOB
            | MYSQL_TYPE_LONG_BLOB
            | MYSQL_TYPE_VAR_STRING
            | MYSQL_TYPE_STRING
                if flags.contains(Flags::ENUM_FLAG) =>
            {
                MysqlType::Enum
            }
            MYSQL_TYPE_BLOB
            | MYSQL_TYPE_TINY_BLOB
            | MYSQL_TYPE_MEDIUM_BLOB
            | MYSQL_TYPE_LONG_BLOB
            | MYSQL_TYPE_VAR_STRING
            | MYSQL_TYPE_STRING
                if flags.contains(Flags::SET_FLAG) =>
            {
                MysqlType::Set
            }

            // "blobs" may contain binary data
            // also "strings" can contain binary data
            // but all only if the binary flag is set
            // (see the check_all_the_types test case)
            MYSQL_TYPE_BLOB
            | MYSQL_TYPE_TINY_BLOB
            | MYSQL_TYPE_MEDIUM_BLOB
            | MYSQL_TYPE_LONG_BLOB
            | MYSQL_TYPE_VAR_STRING
            | MYSQL_TYPE_STRING
                if flags.contains(Flags::BINARY_FLAG) =>
            {
                MysqlType::Blob
            }

            // If the binary flag is not set consider everything as string
            MYSQL_TYPE_BLOB
            | MYSQL_TYPE_TINY_BLOB
            | MYSQL_TYPE_MEDIUM_BLOB
            | MYSQL_TYPE_LONG_BLOB
            | MYSQL_TYPE_VAR_STRING
            | MYSQL_TYPE_STRING => MysqlType::String,

            // unsigned seems to be set for year in any case
            MYSQL_TYPE_YEAR => unreachable!(
                "The year type should have set the unsigned flag. If you ever \
                 see this error message, something has gone very wrong. Please \
                 open an issue at the diesel githup repo in this case"
            ),
            // Null value
            MYSQL_TYPE_NULL => unreachable!(
                "We ensure at the call side that we do not hit this type here. \
                 If you ever see this error, something has gone very wrong. \
                 Please open an issue at the diesel github repo in this case"
            ),
            // Those exist in libmysqlclient
            // but are just not supported
            //
            MYSQL_TYPE_VARCHAR | MYSQL_TYPE_ENUM | MYSQL_TYPE_SET | MYSQL_TYPE_GEOMETRY => {
                unimplemented!(
                    "Hit a type that should be unsupported in libmysqlclient. If \
                     you ever see this error, they probably have added support for \
                     one of those types. Please open an issue at the diesel github \
                     repo in this case."
                )
            }

            MYSQL_TYPE_NEWDATE
            | MYSQL_TYPE_TIME2
            | MYSQL_TYPE_DATETIME2
            | MYSQL_TYPE_TIMESTAMP2 => unreachable!(
                "The mysql documentation states that this types are \
                 only used on server side, so if you see this error \
                 something has gone wrong. Please open a issue at \
                 the diesel github repo."
            ),
        }
    }
}

fn known_buffer_size_for_ffi_type(tpe: ffi::enum_field_types) -> Option<usize> {
    use self::ffi::enum_field_types as t;
    use std::mem::size_of;

    match tpe {
        t::MYSQL_TYPE_TINY => Some(1),
        t::MYSQL_TYPE_YEAR | t::MYSQL_TYPE_SHORT => Some(2),
        t::MYSQL_TYPE_INT24 | t::MYSQL_TYPE_LONG | t::MYSQL_TYPE_FLOAT => Some(4),
        t::MYSQL_TYPE_LONGLONG | t::MYSQL_TYPE_DOUBLE => Some(8),
        t::MYSQL_TYPE_TIME
        | t::MYSQL_TYPE_DATE
        | t::MYSQL_TYPE_DATETIME
        | t::MYSQL_TYPE_TIMESTAMP => Some(size_of::<ffi::MYSQL_TIME>()),
        _ => None,
    }
}

#[allow(warnings)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    use super::MysqlValue;
    use crate::deserialize::FromSql;
    use crate::mysql::connection::stmt::iterator::NamedStatementIterator;
    use crate::sql_types::*;

    fn to_value<ST, T>(
        bind: &BindData,
    ) -> Result<T, Box<(dyn std::error::Error + Send + Sync + 'static)>>
    where
        T: FromSql<ST, crate::mysql::Mysql> + std::fmt::Debug,
    {
        let meta = (bind.tpe, bind.flags).into();
        dbg!(meta);
        let value = MysqlValue::new(&bind.bytes, meta);
        dbg!(T::from_sql(Some(value)))
    }

    #[test]
    fn check_all_the_types() {
        let conn = crate::test_helpers::connection();

        conn.execute("DROP TABLE IF EXISTS all_mysql_types CASCADE")
            .unwrap();
        conn.execute(
            "CREATE TABLE all_mysql_types (
                    tiny_int TINYINT NOT NULL,
                    small_int SMALLINT NOT NULL,
                    medium_int MEDIUMINT NOT NULL,
                    int_col INTEGER NOT NULL,
                    big_int BIGINT NOT NULL,
                    unsigned_int INTEGER UNSIGNED NOT NULL,
                    zero_fill_int INTEGER ZEROFILL NOT NULL,
                    numeric_col NUMERIC(20,5) NOT NULL,
                    decimal_col DECIMAL(20,5) NOT NULL,
                    float_col FLOAT NOT NULL,
                    double_col DOUBLE NOT NULL,
                    bit_col BIT(8) NOT NULL,
                    date_col DATE NOT NULL,
                    date_time DATETIME NOT NULL,
                    timestamp_col TIMESTAMP NOT NULL,
                    time_col TIME NOT NULL,
                    year_col YEAR NOT NULL,
                    char_col CHAR(30) NOT NULL,
                    varchar_col VARCHAR(30) NOT NULL,
                    binary_col BINARY(30) NOT NULL,
                    varbinary_col VARBINARY(30) NOT NULL,
                    blob_col BLOB NOT NULL,
                    text_col TEXT NOT NULL,
                    enum_col ENUM('red', 'green', 'blue') NOT NULL,
                    set_col SET('one', 'two') NOT NULL,
                    geom GEOMETRY NOT NULL,
                    point_col POINT NOT NULL,
                    linestring_col LINESTRING NOT NULL,
                    polygon_col POLYGON NOT NULL,
                    multipoint_col MULTIPOINT NOT NULL,
                    multilinestring_col MULTILINESTRING NOT NULL,
                    multipolygon_col MULTIPOLYGON NOT NULL,
                    geometry_collection GEOMETRYCOLLECTION NOT NULL,
                    json_col JSON NOT NULL
            )",
        )
        .unwrap();
        conn
            .execute(
                "INSERT INTO all_mysql_types VALUES (
                    0, -- tiny_int
                    1, -- small_int
                    2, -- medium_int
                    3, -- int_col
                    -5, -- big_int
                    42, -- unsigned_int
                    1, -- zero_fill_int
                    -999.999, -- numeric_col,
                    3.14, -- decimal_col,
                    1.23, -- float_col
                    4.5678, -- double_col
                    b'10101010', -- bit_col
                    '1000-01-01', -- date_col
                    '9999-12-31 12:34:45.012345', -- date_time
                    '2020-01-01 10:10:10', -- timestamp_col
                    '23:01:01', -- time_col
                    2020, -- year_col
                    'abc', -- char_col
                    'foo', -- varchar_col
                    'a ', -- binary_col
                    'a ', -- varbinary_col
                    'binary', -- blob_col
                    'some text whatever', -- text_col
                    'red', -- enum_col
                    'one', -- set_col
                    ST_GeomFromText('POINT(1 1)'), -- geom
                    ST_PointFromText('POINT(1 1)'), -- point_col
                    ST_LineStringFromText('LINESTRING(0 0,1 1,2 2)'), -- linestring_col
                    ST_PolygonFromText('POLYGON((0 0,10 0,10 10,0 10,0 0),(5 5,7 5,7 7,5 7, 5 5))'), -- polygon_col
                    ST_MultiPointFromText('MULTIPOINT(0 0,10 10,10 20,20 20)'), -- multipoint_col
                    ST_MultiLineStringFromText('MULTILINESTRING((10 48,10 21,10 0),(16 0,16 23,16 48))'), -- multilinestring_col
                    ST_MultiPolygonFromText('MULTIPOLYGON(((28 26,28 0,84 0,84 42,28 26),(52 18,66 23,73 9,48 6,52 18)),((59 18,67 18,67 13,59 13,59 18)))'), -- multipolygon_col
                    ST_GeomCollFromText('GEOMETRYCOLLECTION(POINT(1 1),LINESTRING(0 0,1 1,2 2,3 3,4 4))'), -- geometry_collection
                    '{\"key1\": \"value1\", \"key2\": \"value2\"}' -- json_col
)",
            )
            .unwrap();

        let mut stmt = conn
            .prepare_query(&crate::sql_query(
                "SELECT
                    tiny_int, small_int, medium_int, int_col,
                    big_int, unsigned_int, zero_fill_int,
                    numeric_col, decimal_col, float_col, double_col, bit_col,
                    date_col, date_time, timestamp_col, time_col, year_col,
                    char_col, varchar_col, binary_col, varbinary_col, blob_col,
                    text_col, enum_col, set_col, ST_AsText(geom), ST_AsText(point_col), ST_AsText(linestring_col),
                    ST_AsText(polygon_col), ST_AsText(multipoint_col), ST_AsText(multilinestring_col),
                    ST_AsText(multipolygon_col), ST_AsText(geometry_collection), json_col
                 FROM all_mysql_types",
            ))
            .unwrap();

        let results = unsafe { stmt.named_results().unwrap() };

        let NamedStatementIterator {
            stmt,
            mut output_binds,
            metadata,
        } = results;

        crate::mysql::connection::stmt::iterator::populate_row_buffers(stmt, &mut output_binds)
            .unwrap();

        let results: Vec<(BindData, &ffi::st_mysql_field)> = output_binds
            .data
            .into_iter()
            .zip(metadata.fields())
            .collect::<Vec<_>>();

        macro_rules! matches {
            ($expression:expr, $( $pattern:pat )|+ $( if $guard: expr )?) => {
                match $expression {
                    $( $pattern )|+ $( if $guard )? => true,
                    _ => false
                }
            }
        }

        let tiny_int_col = &results[0].0;
        assert_eq!(tiny_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_TINY);
        assert!(tiny_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!tiny_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<TinyInt, i8>(tiny_int_col), Ok(0)));

        let small_int_col = &results[1].0;
        assert_eq!(small_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_SHORT);
        assert!(small_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!small_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<SmallInt, i16>(small_int_col), Ok(1)));

        let medium_int_col = &results[2].0;
        assert_eq!(medium_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_INT24);
        assert!(medium_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!medium_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<Integer, i32>(medium_int_col), Ok(2)));

        let int_col = &results[3].0;
        assert_eq!(int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG);
        assert!(int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<Integer, i32>(int_col), Ok(3)));

        let big_int_col = &results[4].0;
        assert_eq!(big_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONGLONG);
        assert!(big_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!big_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<TinyInt, i8>(big_int_col), Ok(-5)));

        let unsigned_int_col = &results[5].0;
        assert_eq!(unsigned_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG);
        assert!(unsigned_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(unsigned_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(
            to_value::<Unsigned<Integer>, u32>(unsigned_int_col),
            Ok(42)
        ));

        let zero_fill_int_col = &results[6].0;
        assert_eq!(
            zero_fill_int_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG
        );
        assert!(zero_fill_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(zero_fill_int_col.flags.contains(Flags::ZEROFILL_FLAG));
        assert!(matches!(to_value::<Integer, i32>(zero_fill_int_col), Ok(1)));

        let numeric_col = &results[7].0;
        assert_eq!(
            numeric_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL
        );
        assert!(numeric_col.flags.contains(Flags::NUM_FLAG));
        assert!(!numeric_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert_eq!(
            to_value::<Numeric, bigdecimal::BigDecimal>(numeric_col).unwrap(),
            bigdecimal::BigDecimal::from(-999.999)
        );

        let decimal_col = &results[8].0;
        assert_eq!(
            decimal_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL
        );
        assert!(decimal_col.flags.contains(Flags::NUM_FLAG));
        assert!(!decimal_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert_eq!(
            to_value::<Numeric, bigdecimal::BigDecimal>(decimal_col).unwrap(),
            bigdecimal::BigDecimal::from(3.14)
        );

        let float_col = &results[9].0;
        assert_eq!(float_col.tpe, ffi::enum_field_types::MYSQL_TYPE_FLOAT);
        assert!(float_col.flags.contains(Flags::NUM_FLAG));
        assert!(!float_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<Float, f32>(float_col), Ok(1.23)));

        let double_col = &results[10].0;
        assert_eq!(double_col.tpe, ffi::enum_field_types::MYSQL_TYPE_DOUBLE);
        assert!(double_col.flags.contains(Flags::NUM_FLAG));
        assert!(!double_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<Double, f64>(double_col), Ok(4.5678)));

        let bit_col = &results[11].0;
        assert_eq!(bit_col.tpe, ffi::enum_field_types::MYSQL_TYPE_BIT);
        assert!(!bit_col.flags.contains(Flags::NUM_FLAG));
        assert!(bit_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(!bit_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Blob, Vec<u8>>(bit_col).unwrap(), vec![170]);

        let date_col = &results[12].0;
        assert_eq!(date_col.tpe, ffi::enum_field_types::MYSQL_TYPE_DATE);
        assert!(!date_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Date, chrono::NaiveDate>(date_col).unwrap(),
            chrono::NaiveDate::from_ymd_opt(1000, 1, 1).unwrap(),
        );

        let date_time_col = &results[13].0;
        assert_eq!(
            date_time_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_DATETIME
        );
        assert!(!date_time_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Datetime, chrono::NaiveDateTime>(date_time_col).unwrap(),
            chrono::NaiveDateTime::parse_from_str("9999-12-31 12:34:45", "%Y-%m-%d %H:%M:%S")
                .unwrap()
        );

        let timestamp_col = &results[14].0;
        assert_eq!(
            timestamp_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_TIMESTAMP
        );
        assert!(!timestamp_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Datetime, chrono::NaiveDateTime>(timestamp_col).unwrap(),
            chrono::NaiveDateTime::parse_from_str("2020-01-01 10:10:10", "%Y-%m-%d %H:%M:%S")
                .unwrap()
        );

        let time_col = &results[15].0;
        assert_eq!(time_col.tpe, ffi::enum_field_types::MYSQL_TYPE_TIME);
        assert!(!time_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Time, chrono::NaiveTime>(time_col).unwrap(),
            chrono::NaiveTime::from_hms(23, 01, 01)
        );

        let year_col = &results[16].0;
        assert_eq!(year_col.tpe, ffi::enum_field_types::MYSQL_TYPE_YEAR);
        assert!(year_col.flags.contains(Flags::NUM_FLAG));
        assert!(year_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<SmallInt, i16>(year_col), Ok(2020)));

        let char_col = &results[17].0;
        assert_eq!(char_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!char_col.flags.contains(Flags::NUM_FLAG));
        assert!(!char_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!char_col.flags.contains(Flags::SET_FLAG));
        assert!(!char_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!char_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(char_col).unwrap(), "abc");

        let varchar_col = &results[18].0;
        assert_eq!(
            varchar_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING
        );
        assert!(!varchar_col.flags.contains(Flags::NUM_FLAG));
        assert!(!varchar_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!varchar_col.flags.contains(Flags::SET_FLAG));
        assert!(!varchar_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!varchar_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(varchar_col).unwrap(), "foo");

        let binary_col = &results[19].0;
        assert_eq!(binary_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!binary_col.flags.contains(Flags::NUM_FLAG));
        assert!(!binary_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!binary_col.flags.contains(Flags::SET_FLAG));
        assert!(!binary_col.flags.contains(Flags::ENUM_FLAG));
        assert!(binary_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Blob, Vec<u8>>(binary_col).unwrap(),
            b"a \0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
        );

        let varbinary_col = &results[20].0;
        assert_eq!(
            varbinary_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING
        );
        assert!(!varbinary_col.flags.contains(Flags::NUM_FLAG));
        assert!(!varbinary_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!varbinary_col.flags.contains(Flags::SET_FLAG));
        assert!(!varbinary_col.flags.contains(Flags::ENUM_FLAG));
        assert!(varbinary_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Blob, Vec<u8>>(varbinary_col).unwrap(), b"a ");

        let blob_col = &results[21].0;
        assert_eq!(blob_col.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!blob_col.flags.contains(Flags::NUM_FLAG));
        assert!(blob_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!blob_col.flags.contains(Flags::SET_FLAG));
        assert!(!blob_col.flags.contains(Flags::ENUM_FLAG));
        assert!(blob_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Blob, Vec<u8>>(blob_col).unwrap(), b"binary");

        let text_col = &results[22].0;
        assert_eq!(text_col.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!text_col.flags.contains(Flags::NUM_FLAG));
        assert!(text_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!text_col.flags.contains(Flags::SET_FLAG));
        assert!(!text_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!text_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(text_col).unwrap(),
            "some text whatever"
        );

        let enum_col = &results[23].0;
        assert_eq!(enum_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!enum_col.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col.flags.contains(Flags::SET_FLAG));
        assert!(enum_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(enum_col).unwrap(), "red");

        let set_col = &results[24].0;
        assert_eq!(set_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col.flags.contains(Flags::SET_FLAG));
        assert!(!set_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(set_col).unwrap(), "one");

        let geom = &results[25].0;
        assert_eq!(geom.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!geom.flags.contains(Flags::NUM_FLAG));
        assert!(!geom.flags.contains(Flags::BLOB_FLAG));
        assert!(!geom.flags.contains(Flags::SET_FLAG));
        assert!(!geom.flags.contains(Flags::ENUM_FLAG));
        assert!(!geom.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(geom).unwrap(), "POINT(1 1)");

        let point_col = &results[26].0;
        assert_eq!(point_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!point_col.flags.contains(Flags::NUM_FLAG));
        assert!(!point_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!point_col.flags.contains(Flags::SET_FLAG));
        assert!(!point_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!point_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(point_col).unwrap(), "POINT(1 1)");

        let linestring_col = &results[27].0;
        assert_eq!(
            linestring_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!linestring_col.flags.contains(Flags::NUM_FLAG));
        assert!(!linestring_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!linestring_col.flags.contains(Flags::SET_FLAG));
        assert!(!linestring_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!linestring_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(linestring_col).unwrap(),
            "LINESTRING(0 0,1 1,2 2)"
        );

        let polygon_col = &results[28].0;
        assert_eq!(polygon_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!polygon_col.flags.contains(Flags::NUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!polygon_col.flags.contains(Flags::SET_FLAG));
        assert!(!polygon_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(polygon_col).unwrap(),
            "POLYGON((0 0,10 0,10 10,0 10,0 0),(5 5,7 5,7 7,5 7,5 5))"
        );

        let multipoint_col = &results[29].0;
        assert_eq!(
            multipoint_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!multipoint_col.flags.contains(Flags::NUM_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::SET_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::BINARY_FLAG));
        // older mysql and mariadb versions get back another encoding here
        // we test for both as there seems to be no clear pattern when one or
        // the other is returned
        let multipoint_res = to_value::<Text, String>(multipoint_col).unwrap();
        assert!(
            multipoint_res == "MULTIPOINT((0 0),(10 10),(10 20),(20 20))"
                || multipoint_res == "MULTIPOINT(0 0,10 10,10 20,20 20)"
        );

        let multilinestring_col = &results[30].0;
        assert_eq!(
            multilinestring_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!multilinestring_col.flags.contains(Flags::NUM_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::SET_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(multilinestring_col).unwrap(),
            "MULTILINESTRING((10 48,10 21,10 0),(16 0,16 23,16 48))"
        );

        let polygon_col = &results[31].0;
        assert_eq!(polygon_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!polygon_col.flags.contains(Flags::NUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!polygon_col.flags.contains(Flags::SET_FLAG));
        assert!(!polygon_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(polygon_col).unwrap(),
            "MULTIPOLYGON(((28 26,28 0,84 0,84 42,28 26),(52 18,66 23,73 9,48 6,52 18)),((59 18,67 18,67 13,59 13,59 18)))"
        );

        let geometry_collection = &results[32].0;
        assert_eq!(
            geometry_collection.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!geometry_collection.flags.contains(Flags::NUM_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::BLOB_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::SET_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::ENUM_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(geometry_collection).unwrap(),
            "GEOMETRYCOLLECTION(POINT(1 1),LINESTRING(0 0,1 1,2 2,3 3,4 4))"
        );

        let json_col = &results[33].0;
        // mariadb >= 10.2 and mysql >=8.0 are supporting a json type
        // from those mariadb >= 10.3 and mysql >= 8.0 are reporting
        // json here, so we assert that we get back json
        // mariadb 10.5 returns again blob
        assert!(
            json_col.tpe == ffi::enum_field_types::MYSQL_TYPE_JSON
                || json_col.tpe == ffi::enum_field_types::MYSQL_TYPE_BLOB
        );
        assert!(!json_col.flags.contains(Flags::NUM_FLAG));
        assert!(json_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col.flags.contains(Flags::SET_FLAG));
        assert!(!json_col.flags.contains(Flags::ENUM_FLAG));
        assert!(json_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(json_col).unwrap(),
            "{\"key1\": \"value1\", \"key2\": \"value2\"}"
        );
    }

    fn query_single_table(
        query: &'static str,
        conn: &MysqlConnection,
        bind_tpe: impl Into<(ffi::enum_field_types, Flags)>,
    ) -> BindData {
        let mut stmt: Statement = conn.raw_connection.prepare(query).unwrap();

        let bind = BindData::for_output(bind_tpe.into());

        let mut binds = Binds { data: vec![bind] };

        crate::mysql::connection::stmt::iterator::execute_statement(&mut stmt, &mut binds).unwrap();

        crate::mysql::connection::stmt::iterator::populate_row_buffers(&stmt, &mut binds).unwrap();

        binds.data.remove(0)
    }

    fn input_bind(
        query: &'static str,
        conn: &MysqlConnection,
        id: i32,
        (field, tpe): (Vec<u8>, impl Into<(ffi::enum_field_types, Flags)>),
    ) {
        let mut stmt = conn.raw_connection.prepare(query).unwrap();
        let length = field.len() as _;
        let (tpe, flags) = tpe.into();

        let field_bind = BindData {
            tpe,
            bytes: field,
            length,
            flags,
            is_null: 0,
            is_truncated: None,
        };

        let bytes = id.to_be_bytes().to_vec();
        let length = bytes.len() as _;

        let id_bind = BindData {
            tpe: ffi::enum_field_types::MYSQL_TYPE_LONG,
            bytes,
            length,
            flags: Flags::empty(),
            is_null: 0,
            is_truncated: None,
        };

        let mut binds = Binds {
            data: vec![id_bind, field_bind],
        };

        binds.with_mysql_binds(|bind_ptr| unsafe {
            ffi::mysql_stmt_bind_param(stmt.stmt.as_ptr(), bind_ptr);
        });

        stmt.did_an_error_occur().unwrap();

        let mut out_binds = Binds { data: vec![] };

        unsafe {
            stmt.execute().unwrap();
        }
    }

    #[test]
    fn check_json_bind() {
        let conn: MysqlConnection = crate::test_helpers::connection();

        table! {
            json_test {
                id -> Integer,
                json_field -> Text,
            }
        }

        conn.execute("DROP TABLE IF EXISTS json_test CASCADE")
            .unwrap();

        conn.execute("CREATE TABLE json_test(id INTEGER PRIMARY KEY, json_field JSON NOT NULL)")
            .unwrap();

        conn.execute("INSERT INTO json_test(id, json_field) VALUES (1, '{\"key1\": \"value1\", \"key2\": \"value2\"}')").unwrap();

        let json_col_as_json = query_single_table(
            "SELECT json_field FROM json_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
        );

        assert_eq!(json_col_as_json.tpe, ffi::enum_field_types::MYSQL_TYPE_JSON);
        assert!(!json_col_as_json.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_json).unwrap(),
            "{\"key1\": \"value1\", \"key2\": \"value2\"}"
        );

        let json_col_as_text = query_single_table(
            "SELECT json_field FROM json_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(json_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!json_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_text).unwrap(),
            "{\"key1\": \"value1\", \"key2\": \"value2\"}"
        );
        assert_eq!(json_col_as_json.bytes, json_col_as_text.bytes);

        conn.execute("DELETE FROM json_test").unwrap();

        input_bind(
            "INSERT INTO json_test(id, json_field) VALUES (?, ?)",
            &conn,
            41,
            (
                b"{\"abc\": 42}".to_vec(),
                MysqlType::String,
                //                (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
            ),
        );

        let json_col_as_json = query_single_table(
            "SELECT json_field FROM json_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
        );

        assert_eq!(json_col_as_json.tpe, ffi::enum_field_types::MYSQL_TYPE_JSON);
        assert!(!json_col_as_json.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_json).unwrap(),
            "{\"abc\": 42}"
        );

        let json_col_as_text = query_single_table(
            "SELECT json_field FROM json_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(json_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!json_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_text).unwrap(),
            "{\"abc\": 42}"
        );
        assert_eq!(json_col_as_json.bytes, json_col_as_text.bytes);

        conn.execute("DELETE FROM json_test").unwrap();

        input_bind(
            "INSERT INTO json_test(id, json_field) VALUES (?, ?)",
            &conn,
            41,
            (b"{\"abca\": 42}".to_vec(), MysqlType::String),
        );

        let json_col_as_json = query_single_table(
            "SELECT json_field FROM json_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
        );

        assert_eq!(json_col_as_json.tpe, ffi::enum_field_types::MYSQL_TYPE_JSON);
        assert!(!json_col_as_json.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_json).unwrap(),
            "{\"abca\": 42}"
        );

        let json_col_as_text = query_single_table(
            "SELECT json_field FROM json_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(json_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!json_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_text).unwrap(),
            "{\"abca\": 42}"
        );
        assert_eq!(json_col_as_json.bytes, json_col_as_text.bytes);
    }

    #[test]
    fn check_enum_bind() {
        let conn: MysqlConnection = crate::test_helpers::connection();

        conn.execute("DROP TABLE IF EXISTS enum_test CASCADE")
            .unwrap();

        conn.execute("CREATE TABLE enum_test(id INTEGER PRIMARY KEY, enum_field ENUM('red', 'green', 'blue') NOT NULL)")
            .unwrap();

        conn.execute("INSERT INTO enum_test(id, enum_field) VALUES (1, 'green')")
            .unwrap();

        let enum_col_as_enum: BindData =
            query_single_table("SELECT enum_field FROM enum_test", &conn, MysqlType::Enum);

        assert_eq!(
            enum_col_as_enum.tpe,
            ffi::enum_field_types::MYSQL_TYPE_STRING
        );
        assert!(!enum_col_as_enum.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_enum.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&enum_col_as_enum).unwrap(),
            "green"
        );

        for tpe in &[
            ffi::enum_field_types::MYSQL_TYPE_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING,
            ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB,
        ] {
            let enum_col_as_text = query_single_table(
                "SELECT enum_field FROM enum_test",
                &conn,
                (*tpe, Flags::ENUM_FLAG),
            );

            assert_eq!(enum_col_as_text.tpe, *tpe);
            assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
            assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
            assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
            assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
            assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
            assert_eq!(
                to_value::<Text, String>(&enum_col_as_text).unwrap(),
                "green"
            );
            assert_eq!(enum_col_as_enum.bytes, enum_col_as_text.bytes);
        }

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&enum_col_as_text).unwrap(),
            "green"
        );
        assert_eq!(enum_col_as_enum.bytes, enum_col_as_text.bytes);

        conn.execute("DELETE FROM enum_test").unwrap();

        input_bind(
            "INSERT INTO enum_test(id, enum_field) VALUES (?, ?)",
            &conn,
            41,
            (b"blue".to_vec(), MysqlType::Enum),
        );

        let enum_col_as_enum =
            query_single_table("SELECT enum_field FROM enum_test", &conn, MysqlType::Enum);

        assert_eq!(
            enum_col_as_enum.tpe,
            ffi::enum_field_types::MYSQL_TYPE_STRING
        );
        assert!(!enum_col_as_enum.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_enum.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_enum).unwrap(), "blue");

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_text).unwrap(), "blue");
        assert_eq!(enum_col_as_enum.bytes, enum_col_as_text.bytes);

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_text).unwrap(), "blue");
        assert_eq!(enum_col_as_enum.bytes, enum_col_as_text.bytes);

        conn.execute("DELETE FROM enum_test").unwrap();

        input_bind(
            "INSERT INTO enum_test(id, enum_field) VALUES (?, ?)",
            &conn,
            41,
            (
                b"red".to_vec(),
                (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
            ),
        );

        let enum_col_as_enum =
            query_single_table("SELECT enum_field FROM enum_test", &conn, MysqlType::Enum);

        assert_eq!(
            enum_col_as_enum.tpe,
            ffi::enum_field_types::MYSQL_TYPE_STRING
        );
        assert!(!enum_col_as_enum.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_enum.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_enum).unwrap(), "red");

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_text).unwrap(), "red");
        assert_eq!(enum_col_as_enum.bytes, enum_col_as_text.bytes);
    }

    #[test]
    fn check_set_bind() {
        let conn: MysqlConnection = crate::test_helpers::connection();

        conn.execute("DROP TABLE IF EXISTS set_test CASCADE")
            .unwrap();

        conn.execute("CREATE TABLE set_test(id INTEGER PRIMARY KEY, set_field SET('red', 'green', 'blue') NOT NULL)")
            .unwrap();

        conn.execute("INSERT INTO set_test(id, set_field) VALUES (1, 'green')")
            .unwrap();

        let set_col_as_set: BindData =
            query_single_table("SELECT set_field FROM set_test", &conn, MysqlType::Set);

        assert_eq!(set_col_as_set.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col_as_set.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_set.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_set).unwrap(), "green");

        for tpe in &[
            ffi::enum_field_types::MYSQL_TYPE_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING,
            ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB,
        ] {
            let set_col_as_text = query_single_table(
                "SELECT set_field FROM set_test",
                &conn,
                (*tpe, Flags::SET_FLAG),
            );

            assert_eq!(set_col_as_text.tpe, *tpe);
            assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
            assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
            assert!(set_col_as_text.flags.contains(Flags::SET_FLAG));
            assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
            assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
            assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "green");
            assert_eq!(set_col_as_set.bytes, set_col_as_text.bytes);
        }
        let set_col_as_text = query_single_table(
            "SELECT set_field FROM set_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(set_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "green");
        assert_eq!(set_col_as_set.bytes, set_col_as_text.bytes);

        conn.execute("DELETE FROM set_test").unwrap();

        input_bind(
            "INSERT INTO set_test(id, set_field) VALUES (?, ?)",
            &conn,
            41,
            (b"blue".to_vec(), MysqlType::Set),
        );

        let set_col_as_set =
            query_single_table("SELECT set_field FROM set_test", &conn, MysqlType::Set);

        assert_eq!(set_col_as_set.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col_as_set.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_set.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_set).unwrap(), "blue");

        let set_col_as_text = query_single_table(
            "SELECT set_field FROM set_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::SET_FLAG),
        );

        assert_eq!(set_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "blue");
        assert_eq!(set_col_as_set.bytes, set_col_as_text.bytes);

        conn.execute("DELETE FROM set_test").unwrap();

        input_bind(
            "INSERT INTO set_test(id, set_field) VALUES (?, ?)",
            &conn,
            41,
            (b"red".to_vec(), MysqlType::String),
        );

        let set_col_as_set =
            query_single_table("SELECT set_field FROM set_test", &conn, MysqlType::Set);

        assert_eq!(set_col_as_set.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col_as_set.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_set.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_set).unwrap(), "red");

        let set_col_as_text = query_single_table(
            "SELECT set_field FROM set_test",
            &conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::SET_FLAG),
        );

        assert_eq!(set_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "red");
        assert_eq!(set_col_as_set.bytes, set_col_as_text.bytes);
    }
}
