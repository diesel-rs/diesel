//! This module makes it possible to map `jiff` date and time values to postgres `Date`
//! fields. It is enabled with the `jiff` feature.

use jiff::Span;
use jiff::civil;

use super::PgDate;
use crate::deserialize::{self, Defaultable, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, Output, ToSql};
use crate::sql_types::Date;

// Postgres dates start from January 1st 2000.
const PG_EPOCH_DATE: civil::Date = civil::date(2000, 1, 1);

impl ToSql<Date, Pg> for civil::Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let days_since_epoch = self.since(PG_EPOCH_DATE)?.get_days();
        ToSql::<Date, Pg>::to_sql(&PgDate(days_since_epoch), &mut out.reborrow())
    }
}

impl FromSql<Date, Pg> for civil::Date {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let PgDate(days_since_epoch) = FromSql::<Date, Pg>::from_sql(bytes)?;
        let span = Span::new().try_days(days_since_epoch)?;
        Ok(PG_EPOCH_DATE.checked_add(span)?)
    }
}

impl Defaultable for civil::Date {
    fn default_value() -> Self {
        PG_EPOCH_DATE
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{self, date};

    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::result::Error::{DatabaseError, DeserializationError};
    use crate::select;
    use crate::sql_types::Date;
    use crate::test_helpers::connection;

    #[diesel_test_helper::test]
    fn dates_encode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'").eq(january_first_2000));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_past = date(-398, 4, 11); // jiff uses year 0 for 1 BC
        let query = select(sql::<Date>("'399-4-11 BC'").eq(distant_past));
        assert!(query.get_result::<bool>(connection).unwrap());

        let julian_epoch = date(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date").eq(julian_epoch));
        assert!(query.get_result::<bool>(connection).unwrap());

        let max_date = civil::Date::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date").eq(max_date));
        assert!(query.get_result::<bool>(connection).unwrap());

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date").eq(january_first_2018));
        assert!(query.get_result::<bool>(connection).unwrap());

        let distant_future = date(9999, 1, 8);
        let query = select(sql::<Date>("'9999-1-8'::date").eq(distant_future));
        assert!(query.get_result::<bool>(connection).unwrap());
    }

    #[diesel_test_helper::test]
    fn dates_decode_correctly() {
        let connection = &mut connection();
        let january_first_2000 = date(2000, 1, 1);
        let query = select(sql::<Date>("'2000-1-1'::date"));
        assert_eq!(Ok(january_first_2000), query.get_result(connection));

        let distant_past = date(-398, 4, 11);
        let query = select(sql::<Date>("'399-4-11 BC'::date"));
        assert_eq!(Ok(distant_past), query.get_result(connection));

        let julian_epoch = date(-4713, 11, 24);
        let query = select(sql::<Date>("'J0'::date"));
        assert_eq!(Ok(julian_epoch), query.get_result(connection));

        let max_date = civil::Date::MAX;
        let query = select(sql::<Date>("'9999-12-31'::date"));
        assert_eq!(Ok(max_date), query.get_result(connection));

        let january_first_2018 = date(2018, 1, 1);
        let query = select(sql::<Date>("'2018-1-1'::date"));
        assert_eq!(Ok(january_first_2018), query.get_result(connection));

        let distant_future = date(9999, 1, 8);
        let query = select(sql::<Date>("'9999-1-8'::date"));
        assert_eq!(Ok(distant_future), query.get_result(connection));
    }

    #[diesel_test_helper::test]
    fn dates_outside_of_jiff_range_fail_to_decode() {
        let connection = &mut connection();
        let query = select(sql::<Date>("'10000-1-1'::date"));
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DeserializationError(_))
        ));

        let query = select(sql::<Date>("'infinity'::date"));
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DeserializationError(_))
        ));

        let query = select(sql::<Date>("'-infinity'::date"));
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DeserializationError(_))
        ));
    }

    #[diesel_test_helper::test]
    fn dates_before_postgres_range_fail_to_encode() {
        let connection = &mut connection();
        let query = select(civil::Date::MIN.into_sql::<Date>());
        assert!(matches!(
            query.get_result::<civil::Date>(connection),
            Err(DatabaseError(..))
        ));
    }
}
