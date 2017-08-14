use schema::{connection, TestConnection};
use diesel::*;
use diesel::data_types::*;
use diesel::expression::dsl::*;
use diesel::types::Nullable;

table! {
    has_timestamps {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    has_time {
        id -> Integer,
        time -> Time,
    }
}

#[test]
#[cfg(feature = "postgres")]
fn now_executes_sql_function_now() {
    use self::has_timestamps::dsl::*;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_timestamps (created_at) VALUES
                       (NOW() - '1 day'::interval), (NOW() + '1 day'::interval)")
        .unwrap();

    let before_today = has_timestamps.select(id)
        .filter(created_at.lt(now))
        .load::<i32>(&connection);
    let after_today = has_timestamps.select(id)
        .filter(created_at.gt(now))
        .load::<i32>(&connection);
    assert_eq!(Ok(vec![1]), before_today);
    assert_eq!(Ok(vec![2]), after_today);
}

#[test]
#[cfg(feature = "postgres")]
// FIXME: Replace this with an actual timestamptz expression
fn now_can_be_used_as_timestamptz() {
    use self::has_timestamps::dsl::*;
    use diesel::types::Timestamptz;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_timestamps (created_at) VALUES \
                        (NOW() - '1 day'::interval)").unwrap();

    let created_at_tz = sql::<Timestamptz>("created_at");
    let before_now = has_timestamps.select(id)
        .filter(created_at_tz.lt(now))
        .load::<i32>(&connection);
    assert_eq!(Ok(vec![1]), before_now);
}

#[test]
#[cfg(feature = "postgres")]
// FIXME: Replace this with an actual timestamptz expression
fn now_can_be_used_as_nullable_timestamptz() {
    use self::has_timestamps::dsl::*;
    use diesel::types::Timestamptz;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_timestamps (created_at) VALUES \
                        (NOW() - '1 day'::interval)").unwrap();

    let created_at_tz = sql::<Nullable<Timestamptz>>("created_at");
    let before_now = has_timestamps.select(id)
        .filter(created_at_tz.lt(now))
        .load::<i32>(&connection);
    assert_eq!(Ok(vec![1]), before_now);
}

#[test]
#[cfg(feature = "sqlite")]
fn now_executes_sql_function_now() {
    use self::has_timestamps::dsl::*;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_timestamps (created_at) VALUES
                        (DATETIME('now', '-1 day')), (DATETIME('now', '+1 day'))")
        .unwrap();

    let before_today = has_timestamps.select(id)
        .filter(created_at.lt(now))
        .load::<i32>(&connection);
    let after_today = has_timestamps.select(id)
        .filter(created_at.gt(now))
        .load::<i32>(&connection);
    assert_eq!(Ok(vec![1]), before_today);
    assert_eq!(Ok(vec![2]), after_today);
}


#[test]
#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn date_uses_sql_function_date() {
    use self::has_timestamps::dsl::*;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_timestamps (created_at, updated_at) VALUES
                       ('2015-11-15 06:07:41', '2015-11-15 20:07:41'),
                       ('2015-11-16 06:07:41', '2015-11-17 20:07:41'),
                       ('2015-11-16 06:07:41', '2015-11-16 02:07:41')
                       ").unwrap();

    let expected_data = vec![1, 3];
    let actual_data = has_timestamps.select(id)
        .filter(date(created_at).eq(date(updated_at)))
        .load(&connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(feature = "postgres")]
fn time_is_deserialized_properly() {
    use self::has_time::dsl::*;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_time (\"time\") VALUES
                       ('00:00:01'), ('00:02:00'), ('03:00:00')
                       ").unwrap();
    let one_second = PgTime(1_000_000);
    let two_minutes = PgTime(120_000_000);
    let three_hours = PgTime(10_800_000_000);
    let expected_data = vec![one_second, two_minutes, three_hours];

    let actual_data = has_time.select(time).load(&connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(feature = "postgres")]
fn interval_is_deserialized_properly() {
    use diesel::expression::dsl::sql;
    let connection = connection();

    let data = select(sql::
        <(types::Interval, types::Interval, types::Interval, types::Interval)>(
            "'1 minute'::interval, '1 day'::interval, '1 month'::interval,
                    '4 years 3 days 2 hours 1 minute'::interval"))
        .first(&connection);

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
    use diesel::expression::dsl::sql;

    let connection = connection();
    setup_test_table(&connection);
    connection.execute("INSERT INTO has_timestamps (created_at, updated_at) VALUES
                       ('2015-11-15 06:07:41', '2015-11-15 20:07:41')").unwrap();

    let expected_data = select(sql::<types::Timestamp>("'2015-11-16 06:07:41'::timestamp"))
        .get_result::<PgTimestamp>(&connection);
    let actual_data = has_timestamps.select(created_at + 1.day())
        .first::<PgTimestamp>(&connection);
    assert_eq!(expected_data, actual_data);
}

#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn setup_test_table(conn: &TestConnection) {
    use schema_dsl::*;

    create_table("has_timestamps", (
        integer("id").primary_key().auto_increment(),
        timestamp("created_at").not_null(),
        timestamp("updated_at").not_null().default("CURRENT_TIMESTAMP"),
    )).execute(conn).unwrap();

    create_table("has_time", (
        integer("id").primary_key().auto_increment(),
        time("time").not_null(),
    )).execute(conn).unwrap();
}
