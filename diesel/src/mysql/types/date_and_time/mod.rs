use std::io::Write;
use std::os::raw as libc;
use std::{mem, slice};

use crate::deserialize::{self, FromSql, FromSqlRow};
use crate::expression::AsExpression;
use crate::mysql::{Mysql, MysqlValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{Date, Datetime, Time, Timestamp};

#[cfg(feature = "chrono")]
mod chrono;
#[cfg(feature = "time")]
mod time;

// This is a type from libmysqlclient
// we have our own copy here to not break the
// public API as soon as this type changes
// in the mysqlclient-sys dependency
/// Corresponding rust representation of the
/// [MYSQL_TIME](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html)
/// struct from libmysqlclient
#[repr(C)]
#[derive(Debug, Clone, Copy, AsExpression, FromSqlRow)]
#[non_exhaustive]
#[diesel(sql_type = Timestamp)]
#[diesel(sql_type = Time)]
#[diesel(sql_type = Date)]
#[diesel(sql_type = Datetime)]
pub struct MysqlTime {
    /// [Year field](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#af585231d3ed0bc2fa389856e61e15d4e)
    pub year: libc::c_uint,
    /// [Month field](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#ad3e92bddbd9ccf2e50117bdd51c235a2)
    pub month: libc::c_uint,
    /// [Day field](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#ad51088bd5ab4ddc02e62d778d71ed808)
    pub day: libc::c_uint,
    /// [Hour field](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#a7717a9c4de23a22863fe9c20b0706274)
    pub hour: libc::c_uint,
    /// [Minute field](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#acfad0dafd22da03a527c58fdebfa9d14)
    pub minute: libc::c_uint,
    /// [Second field](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#a4cceb29d1a457f2ea961ce0d893814da)
    pub second: libc::c_uint,
    /// [Microseconds](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#a2e0fddb071af25ff478d16dc5514ba71)
    pub second_part: libc::c_ulong,
    /// [Is this a negative timestamp](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#af13161fbff85e4fe0ec9cd49b6eac1b8)
    pub neg: bool,
    /// [Timestamp type](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#a5331236f9b527a6e6b5f23d7c8058665)
    pub time_type: MysqlTimestampType,
    /// [Time zone displacement specified is seconds](https://dev.mysql.com/doc/dev/mysql-server/latest/structMYSQL__TIME.html#a07f3c8e1989c9805ba919d2120c8fed4)
    pub time_zone_displacement: libc::c_int,
}

impl MysqlTime {
    /// Construct a new instance of [MysqlTime]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: libc::c_uint,
        month: libc::c_uint,
        day: libc::c_uint,
        hour: libc::c_uint,
        minute: libc::c_uint,
        second: libc::c_uint,
        second_part: libc::c_ulong,
        neg: bool,
        time_type: MysqlTimestampType,
        time_zone_displacement: libc::c_int,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            second_part,
            neg,
            time_type,
            time_zone_displacement,
        }
    }
}

// This is a type from libmysqlclient
// we have our own copy here to not break the
// public API as soon as this type changes
// in the mysqlclient-sys dependency
/// Rust representation of
/// [enum_mysql_timestamp_type](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73)
#[derive(PartialEq, Debug, Copy, Clone, Eq)]
#[repr(transparent)]
pub struct MysqlTimestampType(libc::c_int);

impl MysqlTimestampType {
    /// Rust representation of
    /// [MYSQL_TIMESTAMP_NONE](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73ace26c6b7d67a27c905dbcd130b3bd807)
    pub const MYSQL_TIMESTAMP_NONE: MysqlTimestampType = MysqlTimestampType(-2);
    /// Rust representation of
    /// [MYSQL_TIMESTAMP_ERROR](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73a3518624dcc1eaca8d816c52aa7528f72)
    pub const MYSQL_TIMESTAMP_ERROR: MysqlTimestampType = MysqlTimestampType(-1);
    /// Rust representation of
    /// [MYSQL_TIMESTAMP_DATE](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73a9e0845dc169b1f0056d2ffa3780c3f4e)
    pub const MYSQL_TIMESTAMP_DATE: MysqlTimestampType = MysqlTimestampType(0);
    /// Rust representation of
    /// [MYSQL_TIMESTAMP_DATETIME](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73a8f6d8f066ea6ea77280c6a0baf063ce1)
    pub const MYSQL_TIMESTAMP_DATETIME: MysqlTimestampType = MysqlTimestampType(1);
    /// Rust representation of
    /// [MYSQL_TIMESTAMP_TIME](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73a283c50fa3c62a2e17ad5173442edbbb9)
    pub const MYSQL_TIMESTAMP_TIME: MysqlTimestampType = MysqlTimestampType(2);
    /// Rust representation of
    /// [MYSQL_TIMESTAMP_DATETIME_TZ](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__time_8h.html#aa633db8da896a5a0cc00ffcfb7477e73a7afc91f565961eb5f3beebfebe7243a2)
    pub const MYSQL_TIMESTAMP_DATETIME_TZ: MysqlTimestampType = MysqlTimestampType(3);
}

macro_rules! mysql_time_impls {
    ($ty:ty) => {
        #[cfg(feature = "mysql_backend")]
        impl ToSql<$ty, Mysql> for MysqlTime {
            #[allow(unsafe_code)] // pointer cast
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
                let bytes = unsafe {
                    let bytes_ptr = self as *const MysqlTime as *const u8;
                    slice::from_raw_parts(bytes_ptr, mem::size_of::<MysqlTime>())
                };
                out.write_all(bytes)?;
                Ok(IsNull::No)
            }
        }

        #[cfg(feature = "mysql_backend")]
        impl FromSql<$ty, Mysql> for MysqlTime {
            fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
                value.time_value()
            }
        }
    };
}

mysql_time_impls!(Datetime);
mysql_time_impls!(Timestamp);
mysql_time_impls!(Time);
mysql_time_impls!(Date);
