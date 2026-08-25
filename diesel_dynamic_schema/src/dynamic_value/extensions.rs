use diesel::deserialize::{self, FromSql};
use diesel::sql_types as st;

use super::{DynamicDecodeContext, DynamicValueBackend, DynamicValueExtension};

fn declared<DB, ST>(context: &DynamicDecodeContext<'_, DB>) -> bool
where
    DB: DynamicValueBackend,
    ST: st::SqlType,
{
    context
        .declared_sql_type()
        .map(|descriptor| descriptor.is_sql_type::<ST>())
        .unwrap_or(false)
}

fn from_sql<ST, DB, T>(value: DB::RawValue<'_>) -> deserialize::Result<T>
where
    DB: DynamicValueBackend,
    T: FromSql<ST, DB>,
{
    T::from_sql(value)
}

#[cfg(feature = "chrono")]
/// Values decoded by `ChronoExtension`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChronoValue {
    /// A date value.
    Date(chrono::NaiveDate),
    /// A time value.
    Time(chrono::NaiveTime),
    /// A timestamp value.
    Timestamp(chrono::NaiveDateTime),
    /// A timestamp with time zone value.
    Timestamptz(chrono::DateTime<chrono::Utc>),
    /// An interval value.
    Interval(chrono::Duration),
}

#[cfg(feature = "chrono")]
/// Decode date and time values through `chrono`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChronoExtension;

#[cfg(feature = "time")]
/// Values decoded by `TimeExtension`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimeValue {
    /// A date value.
    Date(time::Date),
    /// A time value.
    Time(time::Time),
    /// A timestamp value.
    Timestamp(time::PrimitiveDateTime),
    /// A timestamp with time zone value.
    Timestamptz(time::OffsetDateTime),
}

#[cfg(feature = "time")]
/// Decode date and time values through `time`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeExtension;

#[cfg(feature = "numeric")]
/// Decode exact numeric values through `bigdecimal`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BigDecimalExtension;

#[cfg(feature = "uuid")]
/// Decode UUID values through `uuid`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UuidExtension;

#[cfg(feature = "serde_json")]
/// Decode JSON values through `serde_json`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SerdeJsonExtension;

#[cfg(feature = "network-address")]
/// Decode PostgreSQL network values through `ipnetwork`.
#[derive(Debug, Clone, Copy, Default)]
pub struct IpNetworkExtension;

#[cfg(feature = "ipnet-address")]
/// Decode PostgreSQL network values through `ipnet`.
#[derive(Debug, Clone, Copy, Default)]
pub struct IpNetExtension;

#[cfg(all(feature = "chrono", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for ChronoExtension {
    type Value = ChronoValue;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Date>(context)
            || declared::<_, st::Time>(context)
            || declared::<_, st::Timestamp>(context)
            || declared::<_, st::Timestamptz>(context)
            || declared::<_, st::Interval>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) if matches!(oid.get(), 1082 | 1083 | 1114 | 1184 | 1186))
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Date>(context) {
            return from_sql::<st::Date, diesel::pg::Pg, chrono::NaiveDate>(value)
                .map(ChronoValue::Date);
        }
        if declared::<_, st::Time>(context) {
            return from_sql::<st::Time, diesel::pg::Pg, chrono::NaiveTime>(value)
                .map(ChronoValue::Time);
        }
        if declared::<_, st::Timestamp>(context) {
            return from_sql::<st::Timestamp, diesel::pg::Pg, chrono::NaiveDateTime>(value)
                .map(ChronoValue::Timestamp);
        }
        if declared::<_, st::Timestamptz>(context) {
            return from_sql::<st::Timestamptz, diesel::pg::Pg, chrono::DateTime<chrono::Utc>>(
                value,
            )
            .map(ChronoValue::Timestamptz);
        }
        if declared::<_, st::Interval>(context) {
            return from_sql::<st::Interval, diesel::pg::Pg, chrono::Duration>(value)
                .map(ChronoValue::Interval);
        }
        match context.backend_tag() {
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1082 => {
                from_sql::<st::Date, diesel::pg::Pg, chrono::NaiveDate>(value)
                    .map(ChronoValue::Date)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1083 => {
                from_sql::<st::Time, diesel::pg::Pg, chrono::NaiveTime>(value)
                    .map(ChronoValue::Time)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1114 => {
                from_sql::<st::Timestamp, diesel::pg::Pg, chrono::NaiveDateTime>(value)
                    .map(ChronoValue::Timestamp)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1184 => {
                from_sql::<st::Timestamptz, diesel::pg::Pg, chrono::DateTime<chrono::Utc>>(value)
                    .map(ChronoValue::Timestamptz)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1186 => {
                from_sql::<st::Interval, diesel::pg::Pg, chrono::Duration>(value)
                    .map(ChronoValue::Interval)
            }
            _ => Err("chrono extension did not claim this value".into()),
        }
    }
}

#[cfg(all(feature = "time", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for TimeExtension {
    type Value = TimeValue;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Date>(context)
            || declared::<_, st::Time>(context)
            || declared::<_, st::Timestamp>(context)
            || declared::<_, st::Timestamptz>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) if matches!(oid.get(), 1082 | 1083 | 1114 | 1184))
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Date>(context) {
            return from_sql::<st::Date, diesel::pg::Pg, time::Date>(value).map(TimeValue::Date);
        }
        if declared::<_, st::Time>(context) {
            return from_sql::<st::Time, diesel::pg::Pg, time::Time>(value).map(TimeValue::Time);
        }
        if declared::<_, st::Timestamp>(context) {
            return from_sql::<st::Timestamp, diesel::pg::Pg, time::PrimitiveDateTime>(value)
                .map(TimeValue::Timestamp);
        }
        if declared::<_, st::Timestamptz>(context) {
            return from_sql::<st::Timestamptz, diesel::pg::Pg, time::OffsetDateTime>(value)
                .map(TimeValue::Timestamptz);
        }
        match context.backend_tag() {
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1082 => {
                from_sql::<st::Date, diesel::pg::Pg, time::Date>(value).map(TimeValue::Date)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1083 => {
                from_sql::<st::Time, diesel::pg::Pg, time::Time>(value).map(TimeValue::Time)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1114 => {
                from_sql::<st::Timestamp, diesel::pg::Pg, time::PrimitiveDateTime>(value)
                    .map(TimeValue::Timestamp)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1184 => {
                from_sql::<st::Timestamptz, diesel::pg::Pg, time::OffsetDateTime>(value)
                    .map(TimeValue::Timestamptz)
            }
            _ => Err("time extension did not claim this value".into()),
        }
    }
}

#[cfg(all(feature = "numeric", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for BigDecimalExtension {
    type Value = bigdecimal::BigDecimal;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Numeric>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) if oid.get() == 1700)
    }

    fn decode(
        &self,
        _context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        from_sql::<st::Numeric, diesel::pg::Pg, bigdecimal::BigDecimal>(value)
    }
}

#[cfg(all(feature = "uuid", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for UuidExtension {
    type Value = uuid::Uuid;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Uuid>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) if oid.get() == 2950)
    }

    fn decode(
        &self,
        _context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        from_sql::<st::Uuid, diesel::pg::Pg, uuid::Uuid>(value)
    }
}

#[cfg(all(feature = "serde_json", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for SerdeJsonExtension {
    type Value = serde_json::Value;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Json>(context)
            || declared::<_, st::Jsonb>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) if matches!(oid.get(), 114 | 3802))
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Jsonb>(context) {
            return from_sql::<st::Jsonb, diesel::pg::Pg, serde_json::Value>(value);
        }
        if declared::<_, st::Json>(context) {
            return from_sql::<st::Json, diesel::pg::Pg, serde_json::Value>(value);
        }
        match context.backend_tag() {
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 3802 => {
                from_sql::<st::Jsonb, diesel::pg::Pg, serde_json::Value>(value)
            }
            super::pg::PgTypeTag::Scalar(oid) if oid.get() == 114 => {
                from_sql::<st::Json, diesel::pg::Pg, serde_json::Value>(value)
            }
            _ => Err("serde_json extension did not claim this value".into()),
        }
    }
}

#[cfg(all(feature = "network-address", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for IpNetworkExtension {
    type Value = ipnetwork::IpNetwork;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Inet>(context)
            || declared::<_, st::Cidr>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) | super::pg::PgTypeTag::Other(oid) if matches!(oid.get(), 650 | 869))
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Cidr>(context) {
            return from_sql::<st::Cidr, diesel::pg::Pg, ipnetwork::IpNetwork>(value);
        }
        from_sql::<st::Inet, diesel::pg::Pg, ipnetwork::IpNetwork>(value)
    }
}

#[cfg(all(feature = "ipnet-address", feature = "postgres"))]
impl DynamicValueExtension<diesel::pg::Pg> for IpNetExtension {
    type Value = ipnet::IpNet;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        declared::<_, st::Inet>(context)
            || declared::<_, st::Cidr>(context)
            || matches!(context.backend_tag(), super::pg::PgTypeTag::Scalar(oid) | super::pg::PgTypeTag::Other(oid) if matches!(oid.get(), 650 | 869))
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Cidr>(context) {
            return from_sql::<st::Cidr, diesel::pg::Pg, ipnet::IpNet>(value);
        }
        from_sql::<st::Inet, diesel::pg::Pg, ipnet::IpNet>(value)
    }
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
macro_rules! mysql_like_extension_impls {
    ($db:path) => {
        #[cfg(feature = "chrono")]
        impl DynamicValueExtension<$db> for ChronoExtension {
            type Value = ChronoValue;

            fn claims(&self, context: &DynamicDecodeContext<'_, $db>) -> bool {
                declared::<_, st::Date>(context)
                    || declared::<_, st::Time>(context)
                    || declared::<_, st::Datetime>(context)
                    || declared::<_, st::Timestamp>(context)
                    || matches!(
                        context.backend_tag(),
                        diesel::mysql_like::MysqlType::Date
                            | diesel::mysql_like::MysqlType::Time
                            | diesel::mysql_like::MysqlType::DateTime
                            | diesel::mysql_like::MysqlType::Timestamp
                    )
            }

            fn decode(
                &self,
                context: &DynamicDecodeContext<'_, $db>,
                value: diesel::mysql_like::MysqlValue<'_>,
            ) -> deserialize::Result<Self::Value> {
                if declared::<_, st::Date>(context) {
                    return from_sql::<st::Date, $db, chrono::NaiveDate>(value)
                        .map(ChronoValue::Date);
                }
                if declared::<_, st::Time>(context) {
                    return from_sql::<st::Time, $db, chrono::NaiveTime>(value)
                        .map(ChronoValue::Time);
                }
                if declared::<_, st::Datetime>(context) || declared::<_, st::Timestamp>(context) {
                    return from_sql::<st::Timestamp, $db, chrono::NaiveDateTime>(value)
                        .map(ChronoValue::Timestamp);
                }
                match context.backend_tag() {
                    diesel::mysql_like::MysqlType::Date => {
                        from_sql::<st::Date, $db, chrono::NaiveDate>(value).map(ChronoValue::Date)
                    }
                    diesel::mysql_like::MysqlType::Time => {
                        from_sql::<st::Time, $db, chrono::NaiveTime>(value).map(ChronoValue::Time)
                    }
                    diesel::mysql_like::MysqlType::DateTime
                    | diesel::mysql_like::MysqlType::Timestamp => {
                        from_sql::<st::Timestamp, $db, chrono::NaiveDateTime>(value)
                            .map(ChronoValue::Timestamp)
                    }
                    _ => Err("chrono extension did not claim this value".into()),
                }
            }
        }

        #[cfg(feature = "time")]
        impl DynamicValueExtension<$db> for TimeExtension {
            type Value = TimeValue;

            fn claims(&self, context: &DynamicDecodeContext<'_, $db>) -> bool {
                declared::<_, st::Date>(context)
                    || declared::<_, st::Time>(context)
                    || declared::<_, st::Datetime>(context)
                    || declared::<_, st::Timestamp>(context)
                    || matches!(
                        context.backend_tag(),
                        diesel::mysql_like::MysqlType::Date
                            | diesel::mysql_like::MysqlType::Time
                            | diesel::mysql_like::MysqlType::DateTime
                            | diesel::mysql_like::MysqlType::Timestamp
                    )
            }

            fn decode(
                &self,
                context: &DynamicDecodeContext<'_, $db>,
                value: diesel::mysql_like::MysqlValue<'_>,
            ) -> deserialize::Result<Self::Value> {
                if declared::<_, st::Date>(context) {
                    return from_sql::<st::Date, $db, time::Date>(value).map(TimeValue::Date);
                }
                if declared::<_, st::Time>(context) {
                    return from_sql::<st::Time, $db, time::Time>(value).map(TimeValue::Time);
                }
                if declared::<_, st::Datetime>(context) {
                    return from_sql::<st::Timestamp, $db, time::PrimitiveDateTime>(value)
                        .map(TimeValue::Timestamp);
                }
                if declared::<_, st::Timestamp>(context) {
                    return from_sql::<st::Timestamp, $db, time::PrimitiveDateTime>(value)
                        .map(TimeValue::Timestamp);
                }
                match context.backend_tag() {
                    diesel::mysql_like::MysqlType::Date => {
                        from_sql::<st::Date, $db, time::Date>(value).map(TimeValue::Date)
                    }
                    diesel::mysql_like::MysqlType::Time => {
                        from_sql::<st::Time, $db, time::Time>(value).map(TimeValue::Time)
                    }
                    diesel::mysql_like::MysqlType::DateTime
                    | diesel::mysql_like::MysqlType::Timestamp => {
                        from_sql::<st::Timestamp, $db, time::PrimitiveDateTime>(value)
                            .map(TimeValue::Timestamp)
                    }
                    _ => Err("time extension did not claim this value".into()),
                }
            }
        }

        #[cfg(feature = "numeric")]
        impl DynamicValueExtension<$db> for BigDecimalExtension {
            type Value = bigdecimal::BigDecimal;

            fn claims(&self, context: &DynamicDecodeContext<'_, $db>) -> bool {
                declared::<_, st::Numeric>(context)
                    || matches!(
                        context.backend_tag(),
                        diesel::mysql_like::MysqlType::Numeric
                    )
            }

            fn decode(
                &self,
                _context: &DynamicDecodeContext<'_, $db>,
                value: diesel::mysql_like::MysqlValue<'_>,
            ) -> deserialize::Result<Self::Value> {
                from_sql::<st::Numeric, $db, bigdecimal::BigDecimal>(value)
            }
        }

        #[cfg(feature = "serde_json")]
        impl DynamicValueExtension<$db> for SerdeJsonExtension {
            type Value = serde_json::Value;

            fn claims(&self, context: &DynamicDecodeContext<'_, $db>) -> bool {
                declared::<_, st::Json>(context)
            }

            fn decode(
                &self,
                _context: &DynamicDecodeContext<'_, $db>,
                value: diesel::mysql_like::MysqlValue<'_>,
            ) -> deserialize::Result<Self::Value> {
                from_sql::<st::Json, $db, serde_json::Value>(value)
            }
        }
    };
}

#[cfg(feature = "mysql")]
mysql_like_extension_impls!(diesel::mysql::Mysql);

#[cfg(feature = "mariadb")]
mysql_like_extension_impls!(diesel::mariadb::Mariadb);

#[cfg(all(feature = "chrono", any(feature = "sqlite", feature = "sqlite-no-std")))]
impl DynamicValueExtension<diesel::sqlite::Sqlite> for ChronoExtension {
    type Value = ChronoValue;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>) -> bool {
        declared::<_, st::Date>(context)
            || declared::<_, st::Time>(context)
            || declared::<_, st::Timestamp>(context)
            || declared::<_, st::TimestamptzSqlite>(context)
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>,
        value: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Date>(context) {
            return from_sql::<st::Date, diesel::sqlite::Sqlite, chrono::NaiveDate>(value)
                .map(ChronoValue::Date);
        }
        if declared::<_, st::Time>(context) {
            return from_sql::<st::Time, diesel::sqlite::Sqlite, chrono::NaiveTime>(value)
                .map(ChronoValue::Time);
        }
        if declared::<_, st::Timestamp>(context) {
            return from_sql::<st::Timestamp, diesel::sqlite::Sqlite, chrono::NaiveDateTime>(value)
                .map(ChronoValue::Timestamp);
        }
        if declared::<_, st::TimestamptzSqlite>(context) {
            return from_sql::<
                st::TimestamptzSqlite,
                diesel::sqlite::Sqlite,
                chrono::DateTime<chrono::Utc>,
            >(value)
            .map(ChronoValue::Timestamptz);
        }
        Err("chrono extension did not claim this value".into())
    }
}

#[cfg(all(feature = "time", any(feature = "sqlite", feature = "sqlite-no-std")))]
impl DynamicValueExtension<diesel::sqlite::Sqlite> for TimeExtension {
    type Value = TimeValue;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>) -> bool {
        declared::<_, st::Date>(context)
            || declared::<_, st::Time>(context)
            || declared::<_, st::Timestamp>(context)
            || declared::<_, st::TimestamptzSqlite>(context)
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>,
        value: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Date>(context) {
            return from_sql::<st::Date, diesel::sqlite::Sqlite, time::Date>(value)
                .map(TimeValue::Date);
        }
        if declared::<_, st::Time>(context) {
            return from_sql::<st::Time, diesel::sqlite::Sqlite, time::Time>(value)
                .map(TimeValue::Time);
        }
        if declared::<_, st::Timestamp>(context) {
            return from_sql::<st::Timestamp, diesel::sqlite::Sqlite, time::PrimitiveDateTime>(
                value,
            )
            .map(TimeValue::Timestamp);
        }
        if declared::<_, st::TimestamptzSqlite>(context) {
            return from_sql::<st::TimestamptzSqlite, diesel::sqlite::Sqlite, time::OffsetDateTime>(value).map(TimeValue::Timestamptz);
        }
        Err("time extension did not claim this value".into())
    }
}

#[cfg(all(
    feature = "numeric",
    any(feature = "sqlite", feature = "sqlite-no-std")
))]
impl DynamicValueExtension<diesel::sqlite::Sqlite> for BigDecimalExtension {
    type Value = bigdecimal::BigDecimal;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>) -> bool {
        declared::<_, st::Numeric>(context)
    }

    fn decode(
        &self,
        _context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>,
        value: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> deserialize::Result<Self::Value> {
        from_sql::<st::Numeric, diesel::sqlite::Sqlite, bigdecimal::BigDecimal>(value)
    }
}

#[cfg(all(
    feature = "serde_json",
    any(feature = "sqlite", feature = "sqlite-no-std")
))]
impl DynamicValueExtension<diesel::sqlite::Sqlite> for SerdeJsonExtension {
    type Value = serde_json::Value;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>) -> bool {
        declared::<_, st::Json>(context) || declared::<_, st::Jsonb>(context)
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>,
        value: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> deserialize::Result<Self::Value> {
        if declared::<_, st::Jsonb>(context) {
            return from_sql::<st::Jsonb, diesel::sqlite::Sqlite, serde_json::Value>(value);
        }
        from_sql::<st::Json, diesel::sqlite::Sqlite, serde_json::Value>(value)
    }
}
