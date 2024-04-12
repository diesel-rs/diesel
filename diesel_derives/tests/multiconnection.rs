use crate::schema::users;
use diesel::connection::Instrumentation;
use diesel::prelude::*;

#[derive(diesel::MultiConnection)]
pub enum InferConnection {
    #[cfg(feature = "postgres")]
    Pg(PgConnection),
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConnection),
    #[cfg(feature = "mysql")]
    Mysql(MysqlConnection),
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[test]
fn check_queries_work() {
    let mut conn = establish_connection();

    // checks that this trait is implemented
    conn.set_instrumentation(None::<Box<dyn Instrumentation>>);
    let _ = conn.instrumentation();

    diesel::sql_query(
        "CREATE TEMPORARY TABLE users(\
         id INTEGER NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL)",
    )
    .execute(&mut conn)
    .unwrap();

    conn.begin_test_transaction().unwrap();

    // these are mostly compile pass tests

    // simple query
    let _ = users::table
        .select((users::id, users::name))
        .load::<User>(&mut conn)
        .unwrap();

    // with bind
    let _ = users::table
        .select((users::id, users::name))
        .find(42)
        .load::<User>(&mut conn)
        .unwrap();

    // simple boxed query
    let _ = users::table
        .into_boxed()
        .select((users::id, users::name))
        .load::<User>(&mut conn)
        .unwrap();

    // with bind
    let _ = users::table
        .into_boxed()
        .select((users::id, users::name))
        .filter(users::id.eq(42))
        .load::<User>(&mut conn)
        .unwrap();

    // as_select
    let _ = users::table
        .select(User::as_select())
        .load(&mut conn)
        .unwrap();

    // boxable expression
    let b = Box::new(users::name.eq("John"))
        as Box<
            dyn BoxableExpression<
                users::table,
                self::multi_connection_impl::MultiBackend,
                SqlType = _,
            >,
        >;

    let _ = users::table
        .filter(b)
        .select(users::id)
        .load::<i32>(&mut conn)
        .unwrap();

    // insert
    diesel::insert_into(users::table)
        .values((users::id.eq(42), users::name.eq("John")))
        .execute(&mut conn)
        .unwrap();
    diesel::insert_into(users::table)
        .values(User {
            id: 43,
            name: "Jane".into(),
        })
        .execute(&mut conn)
        .unwrap();
    // update
    diesel::update(users::table)
        .set(users::name.eq("John"))
        .execute(&mut conn)
        .unwrap();
    diesel::update(users::table.find(42))
        .set(User {
            id: 42,
            name: "Jane".into(),
        })
        .execute(&mut conn)
        .unwrap();

    // delete
    diesel::delete(users::table).execute(&mut conn).unwrap();
}

fn establish_connection() -> InferConnection {
    let database_url = if cfg!(feature = "mysql") {
        dotenvy::var("MYSQL_UNIT_TEST_DATABASE_URL").or_else(|_| dotenvy::var("DATABASE_URL"))
    } else if cfg!(feature = "postgres") {
        dotenvy::var("PG_DATABASE_URL").or_else(|_| dotenvy::var("DATABASE_URL"))
    } else {
        Ok(dotenvy::var("DATABASE_URL").unwrap_or_else(|_| ":memory:".to_owned()))
    };
    let database_url = database_url.expect("DATABASE_URL must be set in order to run tests");

    InferConnection::establish(&database_url).unwrap()
}

#[cfg(all(feature = "chrono", feature = "time"))]
fn make_test_table(conn: &mut InferConnection) {
    match conn {
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref mut conn) => {
            diesel::sql_query(
                "CREATE TEMPORARY TABLE type_test( \
                     small_int SMALLINT,\
                     integer INTEGER,\
                     big_int BIGINT,\
                     float FLOAT4,\
                     double FLOAT8,\
                     string TEXT,\
                     blob BYTEA,\
                     timestamp1 TIMESTAMP,\
                     date1 DATE,\
                     time1 TIME,\
                     timestamp2 TIMESTAMP,\
                     date2 DATE,\
                     time2 TIME
                 )",
            )
            .execute(conn)
            .unwrap();
        }
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref mut conn) => {
            diesel::sql_query(
                "CREATE TEMPORARY TABLE type_test( \
                     small_int SMALLINT,\
                     integer INTEGER,\
                     big_int BIGINT,\
                     float FLOAT4,\
                     double FLOAT8,\
                     string TEXT,\
                     blob BLOB,\
                     timestamp1 TIMESTAMP,\
                     date1 DATE,\
                     time1 TIME,\
                     timestamp2 TIMESTAMP,\
                     date2 DATE,\
                     time2 TIME
                 )",
            )
            .execute(conn)
            .unwrap();
        }
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(ref mut conn) => {
            diesel::sql_query(
                "CREATE TEMPORARY TABLE type_test( \
                     `small_int` SMALLINT,\
                     `integer` INT,\
                     `big_int` BIGINT,\
                     `float` FLOAT,\
                     `double` DOUBLE,\
                     `string` TEXT,\
                     `blob` BLOB,\
                     `timestamp1` DATETIME,
                     `date1` DATE,\
                     `time1` TIME,\
                     `timestamp2` DATETIME,
                     `date2` DATE,\
                     `time2` TIME
                 )",
            )
            .execute(conn)
            .unwrap();
        }
    }
}

#[cfg(all(feature = "chrono", feature = "time"))]
#[test]
fn type_checks() {
    use diesel::internal::derives::multiconnection::{chrono, time};

    table! {
        type_test(integer) {
            small_int -> SmallInt,
            integer -> Integer,
            big_int -> BigInt,
            float -> Float,
            double -> Double,
            string -> Text,
            blob -> Blob,
            timestamp1 -> Timestamp,
            time1 -> Time,
            date1 -> Date,
            timestamp2 -> Timestamp,
            time2 -> Time,
            date2 -> Date,
        }
    }

    let mut conn = establish_connection();
    make_test_table(&mut conn);
    conn.begin_test_transaction().unwrap();
    let small_int = 1_i16;
    let integer = 2_i32;
    let big_int = 3_i64;
    let float = 4.0_f32;
    let double = 5.0_f64;
    let string = String::from("bar");
    let blob = vec![1_u8, 2, 3, 4];
    let date1 = chrono::NaiveDate::from_ymd_opt(2023, 08, 17).unwrap();
    let time1 = chrono::NaiveTime::from_hms_opt(07, 50, 12).unwrap();
    let timestamp1 = chrono::NaiveDateTime::new(date1, time1);
    let time2 = time::Time::from_hms(12, 22, 23).unwrap();
    let date2 = time::Date::from_calendar_date(2023, time::Month::August, 26).unwrap();
    let timestamp2 = time::PrimitiveDateTime::new(date2, time2);

    diesel::insert_into(type_test::table)
        .values((
            type_test::small_int.eq(small_int),
            type_test::integer.eq(integer),
            type_test::big_int.eq(big_int),
            type_test::float.eq(float),
            type_test::double.eq(double),
            type_test::string.eq(&string),
            type_test::blob.eq(&blob),
            type_test::timestamp1.eq(timestamp1),
            type_test::time1.eq(time1),
            type_test::date1.eq(date1),
            type_test::timestamp2.eq(timestamp2),
            type_test::time2.eq(time2),
            type_test::date2.eq(date2),
        ))
        .execute(&mut conn)
        .unwrap();

    let result = type_test::table
        .get_result::<(
            i16,                     //0
            i32,                     //1
            i64,                     //2
            f32,                     //3
            f64,                     //4
            String,                  //5
            Vec<u8>,                 //6
            chrono::NaiveDateTime,   //7
            chrono::NaiveTime,       //8
            chrono::NaiveDate,       //9
            time::PrimitiveDateTime, //10
            time::Time,              //11
            time::Date,              //12
        )>(&mut conn)
        .unwrap();

    assert_eq!(small_int, result.0);
    assert_eq!(integer, result.1);
    assert_eq!(big_int, result.2);
    assert_eq!(float, result.3);
    assert_eq!(double, result.4);
    assert_eq!(string, result.5);
    assert_eq!(blob, result.6);
    assert_eq!(timestamp1, result.7);
    assert_eq!(time1, result.8);
    assert_eq!(date1, result.9);
    assert_eq!(timestamp2, result.10);
    assert_eq!(time2, result.11);
    assert_eq!(date2, result.12);
}

#[cfg(all(feature = "chrono", feature = "time"))]
#[test]
fn nullable_type_checks() {
    use diesel::internal::derives::multiconnection::{chrono, time};

    table! {
        type_test(integer) {
            small_int -> Nullable<SmallInt>,
            integer -> Nullable<Integer>,
            big_int -> Nullable<BigInt>,
            float -> Nullable<Float>,
            double -> Nullable<Double>,
            string -> Nullable<Text>,
            blob -> Nullable<Blob>,
            timestamp1 -> Nullable<Timestamp>,
            time1 -> Nullable<Time>,
            date1 -> Nullable<Date>,
            timestamp2 -> Nullable<Timestamp>,
            time2 -> Nullable<Time>,
            date2 -> Nullable<Date>,
        }
    }

    let mut conn = establish_connection();
    make_test_table(&mut conn);
    conn.begin_test_transaction().unwrap();

    let small_int = Some(1_i16);
    let integer = Some(2_i32);
    let big_int = Some(3_i64);
    let float = Some(4.0_f32);
    let double = Some(5.0_f64);
    let string = Some(String::from("bar"));
    let blob = Some(vec![1_u8, 2, 3, 4]);
    let date1 = Some(chrono::NaiveDate::from_ymd_opt(2023, 08, 17).unwrap());
    let time1 = Some(chrono::NaiveTime::from_hms_opt(07, 50, 12).unwrap());
    let timestamp1 = Some(chrono::NaiveDateTime::new(date1.unwrap(), time1.unwrap()));
    let time2 = Some(time::Time::from_hms(12, 22, 23).unwrap());
    let date2 = Some(time::Date::from_calendar_date(2023, time::Month::August, 26).unwrap());
    let timestamp2 = Some(time::PrimitiveDateTime::new(date2.unwrap(), time2.unwrap()));

    diesel::insert_into(type_test::table)
        .values((
            type_test::small_int.eq(small_int),
            type_test::integer.eq(integer),
            type_test::big_int.eq(big_int),
            type_test::float.eq(float),
            type_test::double.eq(double),
            type_test::string.eq(&string),
            type_test::blob.eq(&blob),
            type_test::timestamp1.eq(timestamp1),
            type_test::time1.eq(time1),
            type_test::date1.eq(date1),
            type_test::timestamp2.eq(timestamp2),
            type_test::time2.eq(time2),
            type_test::date2.eq(date2),
        ))
        .execute(&mut conn)
        .unwrap();

    let result = type_test::table
        .get_result::<(
            Option<i16>,
            Option<i32>,
            Option<i64>,
            Option<f32>,
            Option<f64>,
            Option<String>,
            Option<Vec<u8>>,
            Option<chrono::NaiveDateTime>,
            Option<chrono::NaiveTime>,
            Option<chrono::NaiveDate>,
            Option<time::PrimitiveDateTime>,
            Option<time::Time>,
            Option<time::Date>,
        )>(&mut conn)
        .unwrap();

    assert_eq!(small_int, result.0);
    assert_eq!(integer, result.1);
    assert_eq!(big_int, result.2);
    assert_eq!(float, result.3);
    assert_eq!(double, result.4);
    assert_eq!(string, result.5);
    assert_eq!(blob, result.6);
    assert_eq!(timestamp1, result.7);
    assert_eq!(time1, result.8);
    assert_eq!(date1, result.9);
    assert_eq!(timestamp2, result.10);
    assert_eq!(time2, result.11);
    assert_eq!(date2, result.12);

    diesel::delete(type_test::table).execute(&mut conn).unwrap();

    diesel::insert_into(type_test::table)
        .values((
            type_test::small_int.eq(None::<i16>),
            type_test::integer.eq(None::<i32>),
            type_test::big_int.eq(None::<i64>),
            type_test::float.eq(None::<f32>),
            type_test::double.eq(None::<f64>),
            type_test::string.eq(None::<String>),
            type_test::blob.eq(None::<Vec<u8>>),
            type_test::timestamp1.eq(None::<chrono::NaiveDateTime>),
            type_test::time1.eq(None::<chrono::NaiveTime>),
            type_test::date1.eq(None::<chrono::NaiveDate>),
            type_test::timestamp2.eq(None::<time::PrimitiveDateTime>),
            type_test::time2.eq(None::<time::Time>),
            type_test::date2.eq(None::<time::Date>),
        ))
        .execute(&mut conn)
        .unwrap();
    let result = type_test::table
        .get_result::<(
            Option<i16>,
            Option<i32>,
            Option<i64>,
            Option<f32>,
            Option<f64>,
            Option<String>,
            Option<Vec<u8>>,
            Option<chrono::NaiveDateTime>,
            Option<chrono::NaiveTime>,
            Option<chrono::NaiveDate>,
            Option<time::PrimitiveDateTime>,
            Option<time::Time>,
            Option<time::Date>,
        )>(&mut conn)
        .unwrap();
    assert!(result.0.is_none());
    assert!(result.1.is_none());
    assert!(result.2.is_none());
    assert!(result.3.is_none());
    assert!(result.4.is_none());
    assert!(result.5.is_none());
    assert!(result.6.is_none());
    assert!(result.7.is_none());
    assert!(result.8.is_none());
    assert!(result.9.is_none());
    assert!(result.10.is_none());
    assert!(result.11.is_none());
    assert!(result.12.is_none());
}
