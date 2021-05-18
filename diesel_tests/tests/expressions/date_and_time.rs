use crate::schema::*;
#[cfg(feature = "postgres")]
use diesel::data_types::*;
use diesel::dsl::*;
use diesel::sql_types::Nullable;
use diesel::*;

table! {
    has_timestamps {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

#[cfg(feature = "postgres")]
table! {
    has_timestamptzs {
        id -> Integer,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    has_time {
        id -> Integer,
        time -> Time,
    }
}

table! {
    has_date {
        id -> Integer,
        date -> Date,
    }
}

#[cfg(feature = "postgres")]
table! {
    nullable_date_and_time {
        id -> Integer,
        timestamp -> Nullable<Timestamp>,
        timestamptz -> Nullable<Timestamptz>,
        time -> Nullable<Time>,
        date -> Nullable<Date>,
    }
}

#[cfg(not(feature = "postgres"))]
table! {
    nullable_date_and_time {
        id -> Integer,
        timestamp -> Nullable<Timestamp>,
        time -> Nullable<Time>,
        date -> Nullable<Date>,
    }
}

#[test]
#[cfg(feature = "postgres")]
fn now_executes_sql_function_now() {
    use self::has_timestamps::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (created_at) VALUES
                       (NOW() - '1 day'::interval), (NOW() + '1 day'::interval)",
        )
        .unwrap();

    let before_today = has_timestamps
        .select(id)
        .filter(created_at.lt(now))
        .load::<i32>(connection);
    let after_today = has_timestamps
        .select(id)
        .filter(created_at.gt(now))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![1]), before_today);
    assert_eq!(Ok(vec![2]), after_today);
}

#[test]
#[cfg(feature = "postgres")]
// FIXME: Replace this with an actual timestamptz expression
fn now_can_be_used_as_timestamptz() {
    use self::has_timestamps::dsl::*;
    use diesel::sql_types::Timestamptz;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (created_at) VALUES \
             (NOW() - '1 day'::interval)",
        )
        .unwrap();

    let created_at_tz = sql::<Timestamptz>("created_at");
    let before_now = has_timestamps
        .select(id)
        .filter(created_at_tz.lt(now))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![1]), before_now);
}

#[test]
#[cfg(feature = "postgres")]
// FIXME: Replace this with an actual timestamptz expression
fn now_can_be_used_as_nullable_timestamptz() {
    use self::has_timestamps::dsl::*;
    use diesel::sql_types::Timestamptz;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (created_at) VALUES \
             (NOW() - '1 day'::interval)",
        )
        .unwrap();

    let created_at_tz = sql::<Nullable<Timestamptz>>("created_at");
    let before_now = has_timestamps
        .select(id)
        .filter(created_at_tz.lt(now))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![1]), before_now);
}

#[test]
fn now_can_be_used_as_nullable() {
    use diesel::sql_types::Timestamp;

    let nullable_timestamp = sql::<Nullable<Timestamp>>("CURRENT_TIMESTAMP");
    let result = select(nullable_timestamp.eq(now)).get_result(&mut connection());

    assert_eq!(Ok(Some(true)), result);
}

#[test]
fn today_can_be_used_as_nullable() {
    use diesel::sql_types::Date;

    let nullable_date = sql::<Nullable<Date>>("CURRENT_DATE");
    let result = select(nullable_date.eq(today)).get_result(&mut connection());

    assert_eq!(Ok(Some(true)), result);
}

#[test]
#[cfg(feature = "postgres")]
fn today_executes_sql_function_current_date() {
    use self::has_date::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_date (date) VALUES
                (current_date - '1 day'::interval), (current_date + '1 day'::interval);",
        )
        .unwrap();

    let before_today = has_date
        .select(id)
        .filter(date.lt(today))
        .load::<i32>(connection);
    let after_today = has_date
        .select(id)
        .filter(date.gt(today))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![1]), before_today);
    assert_eq!(Ok(vec![2]), after_today);
}

#[test]
#[cfg(feature = "sqlite")]
fn today_executes_sql_function_current_date() {
    use self::has_date::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_date (date) VALUES
                (DATE('now', '-1 day')), (DATE('now', '+1 day'));",
        )
        .unwrap();

    let before_today = has_date
        .select(id)
        .filter(date.lt(today))
        .load::<i32>(connection);
    let after_today = has_date
        .select(id)
        .filter(date.gt(today))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![1]), before_today);
    assert_eq!(Ok(vec![2]), after_today);
}

#[test]
#[cfg(feature = "mysql")]
fn today_executes_sql_function_current_date() {
    use self::has_date::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_date (id, date) VALUES
                (42, current_date - interval 1 day), (43, current_date + interval 1 day);",
        )
        .unwrap();

    let before_today = has_date
        .select(id)
        .filter(date.lt(today))
        .load::<i32>(connection);
    let after_today = has_date
        .select(id)
        .filter(date.gt(today))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![42]), before_today);
    assert_eq!(Ok(vec![43]), after_today);
}

#[test]
#[cfg(feature = "mysql")]
fn now_executes_sql_function_now() {
    use self::has_timestamps::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (id, created_at) VALUES
                       (42, NOW() - interval 1 day), (43, NOW() + interval 1 day)",
        )
        .unwrap();

    let before_today = has_timestamps.select(id).filter(created_at.lt(now));

    dbg!(diesel::debug_query(&before_today));
    let before_today = before_today.load::<i32>(connection);

    let after_today = has_timestamps.select(id).filter(created_at.gt(now));

    dbg!(diesel::debug_query(&after_today));
    let after_today = after_today.load::<i32>(connection);

    assert_eq!(Ok(vec![42]), before_today);
    assert_eq!(Ok(vec![43]), after_today);
}

#[test]
#[cfg(feature = "sqlite")]
fn now_executes_sql_function_now() {
    use self::has_timestamps::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (created_at) VALUES
                        (DATETIME('now', '-1 day')), (DATETIME('now', '+1 day'))",
        )
        .unwrap();

    let before_today = has_timestamps
        .select(id)
        .filter(created_at.lt(now))
        .load::<i32>(connection);
    let after_today = has_timestamps
        .select(id)
        .filter(created_at.gt(now))
        .load::<i32>(connection);
    assert_eq!(Ok(vec![1]), before_today);
    assert_eq!(Ok(vec![2]), after_today);
}
#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn date_uses_sql_function_date() {
    use self::has_timestamps::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (created_at, updated_at) VALUES
                       ('2015-11-15 06:07:41', '2015-11-15 20:07:41'),
                       ('2015-11-16 06:07:41', '2015-11-17 20:07:41'),
                       ('2015-11-16 06:07:41', '2015-11-16 02:07:41')
                       ",
        )
        .unwrap();

    let expected_data = vec![1, 3];
    let actual_data = has_timestamps
        .select(id)
        .filter(date(created_at).eq(date(updated_at)))
        .load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(feature = "postgres")]
fn time_is_deserialized_properly() {
    use self::has_time::dsl::*;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_time (\"time\") VALUES
                       ('00:00:01'), ('00:02:00'), ('03:00:00')
                       ",
        )
        .unwrap();
    let one_second = PgTime(1_000_000);
    let two_minutes = PgTime(120_000_000);
    let three_hours = PgTime(10_800_000_000);
    let expected_data = vec![one_second, two_minutes, three_hours];

    let actual_data = has_time.select(time).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(feature = "postgres")]
fn interval_is_deserialized_properly() {
    use diesel::dsl::sql;
    let connection = &mut connection();

    let data = select(sql::<(
        sql_types::Interval,
        sql_types::Interval,
        sql_types::Interval,
        sql_types::Interval,
    )>(
        "'1 minute'::interval, '1 day'::interval, '1 month'::interval,
                    '4 years 3 days 2 hours 1 minute'::interval",
    ))
    .first(connection);

    let one_minute = 1.minute();
    let one_day = 1.day();
    let one_month = 1.month();
    let long_time = 4.years() + 3.days() + 2.hours() + 1.minute();
    let expected_data = (one_minute, one_day, one_month, long_time);
    assert_eq!(Ok(expected_data), data);
}

#[test]
#[cfg(feature = "postgres")]
fn adding_interval_to_timestamp() {
    use self::has_timestamps::dsl::*;
    use diesel::dsl::sql;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamps (created_at, updated_at) VALUES
                       ('2015-11-15 06:07:41', '2015-11-15 20:07:41')",
        )
        .unwrap();

    let expected_data = select(sql::<sql_types::Timestamp>(
        "'2015-11-16 06:07:41'::timestamp",
    ))
    .get_result::<PgTimestamp>(connection);
    let actual_data = has_timestamps
        .select(created_at + 1.day())
        .first::<PgTimestamp>(connection);
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(feature = "postgres")]
fn adding_interval_to_timestamptz() {
    use self::has_timestamptzs::dsl::*;
    use diesel::dsl::sql;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO has_timestamptzs (created_at, updated_at) VALUES
                       ('2015-11-15 06:07:41+0100', '2015-11-15 20:07:41+0100')",
        )
        .unwrap();

    let expected_data = select(sql::<sql_types::Timestamptz>(
        "'2015-11-16 06:07:41+0100'::timestamptz",
    ))
    .get_result::<PgTimestamp>(connection);
    let actual_data = has_timestamptzs
        .select(created_at + 1.day())
        .first::<PgTimestamp>(connection);
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(feature = "postgres")]
fn adding_interval_to_nullable_things() {
    use self::nullable_date_and_time::dsl::*;
    use diesel::dsl::sql;

    let connection = &mut connection();
    setup_test_table(connection);
    connection
        .execute(
            "INSERT INTO nullable_date_and_time (timestamp, timestamptz, date, time) VALUES
                       ('2017-08-20 18:13:37', '2017-08-20 18:13:37+0100', '2017-08-20', '18:13:37')",
        )
        .unwrap();

    let expected_data = select(sql::<Nullable<sql_types::Timestamp>>(
        "'2017-08-21 18:13:37'::timestamp",
    ))
    .get_result::<Option<PgTimestamp>>(connection);
    let actual_data = nullable_date_and_time
        .select(timestamp + 1.day())
        .first::<Option<PgTimestamp>>(connection);
    assert_eq!(expected_data, actual_data);

    let expected_data = select(sql::<Nullable<sql_types::Timestamptz>>(
        "'2017-08-21 18:13:37+0100'::timestamptz",
    ))
    .get_result::<Option<PgTimestamp>>(connection);
    let actual_data = nullable_date_and_time
        .select(timestamptz + 1.day())
        .first::<Option<PgTimestamp>>(connection);
    assert_eq!(expected_data, actual_data);

    let expected_data = select(sql::<Nullable<sql_types::Timestamp>>(
        "'2017-08-21'::timestamp",
    ))
    .get_result::<Option<PgTimestamp>>(connection);
    let actual_data = nullable_date_and_time
        .select(date + 1.day())
        .first::<Option<PgTimestamp>>(connection);
    assert_eq!(expected_data, actual_data);

    let expected_data = select(sql::<Nullable<sql_types::Time>>("'19:13:37'::time"))
        .get_result::<Option<PgTime>>(connection);
    let actual_data = nullable_date_and_time
        .select(time + 1.hour())
        .first::<Option<PgTime>>(connection);
    assert_eq!(expected_data, actual_data);
}

#[cfg(any(feature = "postgres", feature = "sqlite", feature = "mysql"))]
fn setup_test_table(conn: &mut TestConnection) {
    use crate::schema_dsl::*;

    create_table(
        "has_timestamps",
        (
            integer("id").primary_key().auto_increment(),
            timestamp("created_at").not_null(),
            timestamp("updated_at")
                .not_null()
                .default("CURRENT_TIMESTAMP"),
        ),
    )
    .execute(conn)
    .unwrap();

    #[cfg(feature = "postgres")]
    create_table(
        "has_timestamptzs",
        (
            integer("id").primary_key().auto_increment(),
            timestamptz("created_at").not_null(),
            timestamptz("updated_at")
                .not_null()
                .default("CURRENT_TIMESTAMP"),
        ),
    )
    .execute(conn)
    .unwrap();

    create_table(
        "has_time",
        (
            integer("id").primary_key().auto_increment(),
            time("time").not_null(),
        ),
    )
    .execute(conn)
    .unwrap();

    create_table(
        "has_date",
        (
            integer("id").primary_key().auto_increment(),
            date("date").not_null(),
        ),
    )
    .execute(conn)
    .unwrap();

    create_table(
        "nullable_date_and_time",
        (
            integer("id").primary_key().auto_increment(),
            timestamp("timestamp"),
            #[cfg(feature = "postgres")]
            timestamptz("timestamptz"),
            time("time"),
            date("date"),
        ),
    )
    .execute(conn)
    .unwrap();

    sql_query("DELETE FROM has_timestamps")
        .execute(conn)
        .unwrap();
    #[cfg(feature = "postgres")]
    sql_query("DELETE FROM has_timestamptzs")
        .execute(conn)
        .unwrap();
    sql_query("DELETE FROM has_date").execute(conn).unwrap();
    sql_query("DELETE FROM nullable_date_and_time")
        .execute(conn)
        .unwrap();
    sql_query("DELETE FROM has_time").execute(conn).unwrap();
}
