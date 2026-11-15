//! This module makes it possible to map `jiff` date and time values to MySQL `Date`
//! fields. It is enabled with the `jiff` feature.

use jiff::civil;

use super::{MysqlTime, MysqlTimestampType};
use crate::deserialize::{self, FromSql};
use crate::mysql_like::{MysqlLikeBackend, MysqlValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::Date;

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
    use jiff::civil::{self, date};

    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::select;
    use crate::sql_types::Date;
    use crate::test_helpers::connection;

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
