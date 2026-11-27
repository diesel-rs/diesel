//! This module makes it possible to map `jiff` date and time values to MySQL `Date`,
//! `Time`, `Datetime` and `Timestamp` fields. It is enabled with the `jiff` feature.

use jiff::civil;

use super::{MysqlTime, MysqlTimestampType};
use crate::deserialize::{self, FromSql};
use crate::mysql_like::{MysqlLikeBackend, MysqlValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::{Date, Datetime, Time, Timestamp};

impl<DB: MysqlLikeBackend> ToSql<Datetime, DB> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year().try_into()?,
            month: self.month().try_into()?,
            day: self.day().try_into()?,
            hour: self.hour().try_into()?,
            minute: self.minute().try_into()?,
            second: self.second().try_into()?,
            second_part: (self.subsec_nanosecond() / 1_000).try_into()?,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Datetime, DB>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

impl<DB: MysqlLikeBackend> FromSql<Datetime, DB> for civil::DateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Datetime, DB>>::from_sql(bytes)?;
        if mysql_time.time_zone_displacement != 0 {
            return Err(format!(
                "Unable to convert {mysql_time:?} to jiff: \
                 values with a time zone cannot be deserialized as jiff::civil::DateTime"
            )
            .into());
        }

        let subsec_nanosecond = i32::try_from(mysql_time.second_part)?.saturating_mul(1_000);
        civil::DateTime::new(
            mysql_time.year.try_into()?,
            mysql_time.month.try_into()?,
            mysql_time.day.try_into()?,
            mysql_time.hour.try_into()?,
            mysql_time.minute.try_into()?,
            mysql_time.second.try_into()?,
            subsec_nanosecond,
        )
        .map_err(|err| format!("Unable to convert {mysql_time:?} to jiff: {err}").into())
    }
}

impl<DB: MysqlLikeBackend> ToSql<Timestamp, DB> for civil::DateTime {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        <civil::DateTime as ToSql<Datetime, DB>>::to_sql(self, out)
    }
}

impl<DB: MysqlLikeBackend> FromSql<Timestamp, DB> for civil::DateTime {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        <civil::DateTime as FromSql<Datetime, DB>>::from_sql(bytes)
    }
}

impl<DB: MysqlLikeBackend> ToSql<Time, DB> for civil::Time {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: 0,
            month: 0,
            day: 0,
            hour: self.hour().try_into()?,
            minute: self.minute().try_into()?,
            second: self.second().try_into()?,
            second_part: (self.subsec_nanosecond() / 1_000).try_into()?,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_TIME,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Time, DB>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

impl<DB: MysqlLikeBackend> FromSql<Time, DB> for civil::Time {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Time, DB>>::from_sql(bytes)?;
        if mysql_time.neg {
            return Err("Negative times cannot be deserialized as jiff::civil::Time".into());
        }

        let subsec_nanosecond = i32::try_from(mysql_time.second_part)?.saturating_mul(1_000);
        civil::Time::new(
            mysql_time.hour.try_into()?,
            mysql_time.minute.try_into()?,
            mysql_time.second.try_into()?,
            subsec_nanosecond,
        )
        .map_err(|err| format!("Unable to convert {mysql_time:?} to jiff: {err}").into())
    }
}

impl<DB: MysqlLikeBackend> ToSql<Date, DB> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let mysql_time = MysqlTime {
            year: self.year().try_into()?,
            month: self.month().try_into()?,
            day: self.day().try_into()?,
            hour: 0,
            minute: 0,
            second: 0,
            second_part: 0,
            neg: false,
            time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATE,
            time_zone_displacement: 0,
        };

        <MysqlTime as ToSql<Date, DB>>::to_sql(&mysql_time, &mut out.reborrow())
    }
}

impl<DB: MysqlLikeBackend> FromSql<Date, DB> for civil::Date {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let mysql_time = <MysqlTime as FromSql<Date, DB>>::from_sql(bytes)?;
        civil::Date::new(
            mysql_time.year.try_into()?,
            mysql_time.month.try_into()?,
            mysql_time.day.try_into()?,
        )
        .map_err(|err| format!("Unable to convert {mysql_time:?} to jiff: {err}").into())
    }
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;
    use jiff::civil::{self, date, datetime, time};
    use jiff::tz::Offset;

    use crate::dsl::{now, sql};
    use crate::prelude::*;
    use crate::result::Error::DeserializationError;
    use crate::select;
    use crate::sql_types::{Date, Datetime, Time, Timestamp};
    use crate::test_helpers::connection;

    #[diesel_test_helper::test]
    fn unix_epoch_encodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let query = select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
        let query = select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)").eq(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn unix_epoch_decodes_correctly() {
        let connection = &mut connection();
        let time = datetime(1970, 1, 1, 0, 0, 0, 0);
        let epoch_from_sql =
            select(sql::<Timestamp>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
        let epoch_from_sql =
            select(sql::<Datetime>("CAST('1970-01-01' AS DATETIME)")).get_result(connection);
        assert_eq!(Ok(time), epoch_from_sql);
    }

    #[diesel_test_helper::test]
    fn times_relative_to_now_encode_correctly() {
        let connection = &mut connection();
        let naive_now = Offset::UTC.to_datetime(jiff::Timestamp::now());
        let time = naive_now + SignedDuration::from_hours(24);
        let query = select(now.lt(time));
        assert!(query.get_result::<bool>(connection).unwrap());

        let time = naive_now - SignedDuration::from_hours(24);
        let query = select(now.gt(time));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn datetimes_with_microseconds_round_trip() {
        let connection = &mut connection();
        let with_micros = datetime(2018, 1, 1, 8, 30, 0, 123_456_000);
        let text = "CAST('2018-01-01 08:30:00.123456' AS DATETIME(6))";
        let query = select(sql::<Datetime>(text).eq(with_micros));
        assert!(query.get_result::<bool>(connection).unwrap());

        let query = select(sql::<Datetime>(text));
        assert_eq!(Ok(with_micros), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn zero_datetime_fails_to_decode() {
        let connection = &mut connection();
        crate::sql_query("SET sql_mode = (SELECT REPLACE(@@sql_mode, 'NO_ZERO_DATE,', ''))")
            .execute(connection)
            .unwrap();
        let query = select(sql::<Datetime>("CAST('0000-00-00 00:00:00' AS DATETIME)"));
        assert!(matches!(
            query.get_result::<civil::DateTime>(connection),
            Err(DeserializationError(_))
        ));
    }

    #[diesel_test_helper::test]
    fn times_of_day_encode_correctly() {
        let connection = &mut connection();
        let midnight = time(0, 0, 0, 0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)").eq(midnight));
        assert!(query.get_result::<bool>(connection).unwrap());

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)").eq(noon));
        assert!(query.get_result::<bool>(connection).unwrap());

        let roughly_half_past_eleven = time(23, 37, 4, 0);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)").eq(roughly_half_past_eleven));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_decode_correctly() {
        let connection = &mut connection();
        let midnight = time(0, 0, 0, 0);
        let query = select(sql::<Time>("CAST('00:00:00' AS TIME)"));
        assert_eq!(Ok(midnight), query.get_result(connection));

        let noon = time(12, 0, 0, 0);
        let query = select(sql::<Time>("CAST('12:00:00' AS TIME)"));
        assert_eq!(Ok(noon), query.get_result(connection));

        let roughly_half_past_eleven = time(23, 37, 4, 0);
        let query = select(sql::<Time>("CAST('23:37:04' AS TIME)"));
        assert_eq!(Ok(roughly_half_past_eleven), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn times_of_day_with_microseconds_encode_correctly() {
        let connection = &mut connection();
        let with_micros = time(8, 30, 0, 123_456_000);
        let query = select(sql::<Time>("CAST('08:30:00.123456' AS TIME(6))").eq(with_micros));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn times_of_day_with_microseconds_decode_correctly() {
        let connection = &mut connection();
        let with_micros = time(8, 30, 0, 123_456_000);
        let query = select(sql::<Time>("CAST('08:30:00.123456' AS TIME(6))"));
        assert_eq!(Ok(with_micros), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn times_outside_of_a_day_fail_to_decode() {
        let connection = &mut connection();
        let query = select(sql::<Time>("CAST('24:00:00' AS TIME)"));
        assert!(matches!(
            query.get_result::<civil::Time>(connection),
            Err(DeserializationError(_))
        ));

        let query = select(sql::<Time>("CAST('-01:00:00' AS TIME)"));
        assert!(matches!(
            query.get_result::<civil::Time>(connection),
            Err(DeserializationError(_))
        ));
    }

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("CAST('2000-1-1' AS DATE)"));
        assert_eq!(Ok(january_first_2000), query.get_result(connection));

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("CAST('2018-1-1' AS DATE)"));
        assert_eq!(Ok(january_first_2018), query.get_result(connection));

        crate::sql_query("SET sql_mode = (SELECT REPLACE(@@sql_mode, 'NO_ZERO_DATE,', ''))")
            .execute(connection)
            .unwrap();
        let query = select(sql::<Date>("CAST('0000-00-00' AS DATE)"));
        assert!(query.get_result::<civil::Date>(connection).is_err());
    }
}
