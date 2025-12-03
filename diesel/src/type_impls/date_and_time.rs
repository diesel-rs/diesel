#![allow(dead_code)]

#[cfg(feature = "std")]
mod std {
    use crate::deserialize::FromSqlRow;
    use crate::expression::AsExpression;

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = crate::sql_types::Timestamp)]
    struct SystemTimeProxy(std::time::SystemTime);
}

#[cfg(feature = "chrono")]
mod chrono {
    extern crate chrono;
    use self::chrono::*;
    use crate::deserialize::FromSqlRow;
    use crate::expression::AsExpression;
    use crate::sql_types::{Date, Interval, Time, Timestamp};

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Date)]
    struct NaiveDateProxy(NaiveDate);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Time)]
    struct NaiveTimeProxy(NaiveTime);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Timestamp)]
    #[cfg_attr(
        feature = "postgres_backend",
        diesel(sql_type = crate::sql_types::Timestamptz)
    )]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::Datetime))]
    struct NaiveDateTimeProxy(NaiveDateTime);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    #[cfg_attr(
        any(feature = "postgres_backend", feature = "__sqlite-shared"),
        derive(AsExpression)
    )]
    #[cfg_attr(
        feature = "postgres_backend",
        diesel(sql_type = crate::sql_types::Timestamptz)
    )]
    #[cfg_attr(feature = "__sqlite-shared", diesel(sql_type = crate::sql_types::TimestamptzSqlite))]
    struct DateTimeProxy<Tz: TimeZone>(DateTime<Tz>);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Interval)]
    struct DurationProxy(Duration);
}

#[cfg(feature = "time")]
mod time {
    use time::{Date as NaiveDate, OffsetDateTime, PrimitiveDateTime, Time as NaiveTime};

    use crate::deserialize::FromSqlRow;
    use crate::expression::AsExpression;
    use crate::sql_types::{Date, Time, Timestamp};

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Date)]
    struct NaiveDateProxy(NaiveDate);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Time)]
    struct NaiveTimeProxy(NaiveTime);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Timestamp)]
    #[cfg_attr(
        feature = "postgres_backend",
        diesel(sql_type = crate::sql_types::Timestamptz)
    )]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::Datetime))]
    struct NaiveDateTimeProxy(PrimitiveDateTime);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    #[cfg_attr(
        any(
            feature = "postgres_backend",
            feature = "__sqlite-shared",
            feature = "mysql_backend"
        ),
        derive(AsExpression)
    )]
    #[cfg_attr(
        feature = "postgres_backend",
        diesel(sql_type = crate::sql_types::Timestamptz)
    )]
    #[cfg_attr(feature = "__sqlite-shared", diesel(sql_type = crate::sql_types::TimestamptzSqlite))]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::Datetime))]
    struct DateTimeProxy(OffsetDateTime);
}
