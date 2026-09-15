use diesel::deserialize::*;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;
use diesel_dynamic_schema::dynamic_value::*;
use diesel_dynamic_schema::DynamicSelectClause;

#[derive(PartialEq, Debug)]
enum MyDynamicValue {
    String(String),
    Integer(i32),
}

#[cfg(feature = "postgres")]
impl FromSql<Any, diesel::pg::Pg> for MyDynamicValue {
    fn from_sql(value: diesel::pg::PgValue) -> Result<Self> {
        use diesel::pg::Pg;
        use std::num::NonZeroU32;

        const VARCHAR_OID: NonZeroU32 = NonZeroU32::new(1043).unwrap();
        const TEXT_OID: NonZeroU32 = NonZeroU32::new(25).unwrap();
        const INTEGER_OID: NonZeroU32 = NonZeroU32::new(23).unwrap();

        match value.get_oid() {
            VARCHAR_OID | TEXT_OID => {
                <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            INTEGER_OID => <i32 as FromSql<diesel::sql_types::Integer, Pg>>::from_sql(value)
                .map(MyDynamicValue::Integer),
            e => Err(format!("Unknown type: {e}").into()),
        }
    }
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
impl FromSql<Any, diesel::sqlite::Sqlite> for MyDynamicValue {
    fn from_sql(value: diesel::sqlite::SqliteValue) -> Result<Self> {
        use diesel::sqlite::{Sqlite, SqliteType};
        match value.value_type() {
            Some(SqliteType::Text) => {
                <String as FromSql<diesel::sql_types::Text, Sqlite>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            Some(SqliteType::Long) => {
                <i32 as FromSql<diesel::sql_types::Integer, Sqlite>>::from_sql(value)
                    .map(MyDynamicValue::Integer)
            }
            _ => Err("Unknown data type".into()),
        }
    }
}

#[cfg(feature = "mysql")]
impl FromSql<Any, diesel::mysql::Mysql> for MyDynamicValue {
    fn from_sql(value: diesel::mysql::MysqlValue) -> Result<Self> {
        use diesel::mysql::{Mysql, MysqlType};
        match value.value_type() {
            MysqlType::String => {
                <String as FromSql<diesel::sql_types::Text, Mysql>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            MysqlType::Long => <i32 as FromSql<diesel::sql_types::Integer, Mysql>>::from_sql(value)
                .map(MyDynamicValue::Integer),
            e => Err(format!("Unknown data type: {e:?}").into()),
        }
    }
}

#[cfg(feature = "mariadb")]
impl FromSql<Any, diesel::mariadb::Mariadb> for MyDynamicValue {
    fn from_sql(value: diesel::mariadb::MariadbValue) -> Result<Self> {
        use diesel::mariadb::{Mariadb, MariadbType};
        match value.value_type() {
            MariadbType::String => {
                <String as FromSql<diesel::sql_types::Text, Mariadb>>::from_sql(value)
                    .map(MyDynamicValue::String)
            }
            MariadbType::Long => {
                <i32 as FromSql<diesel::sql_types::Integer, Mariadb>>::from_sql(value)
                    .map(MyDynamicValue::Integer)
            }
            e => Err(format!("Unknown data type: {e:?}").into()),
        }
    }
}

#[cfg(feature = "postgres")]
type TestDB = diesel::pg::Pg;
#[cfg(feature = "mysql")]
type TestDB = diesel::mysql::Mysql;
#[cfg(feature = "mariadb")]
type TestDB = diesel::mariadb::Mariadb;
#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
type TestDB = diesel::sqlite::Sqlite;

#[test]
#[cfg(any(
    feature = "postgres",
    feature = "mysql",
    feature = "mariadb",
    feature = "sqlite",
    feature = "sqlite-no-std"
))]
fn test_ergonomics() {
    let connection = &mut super::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let name = users.column::<Untyped, _>("name");

    // Test DynamicSelectClause: Extend & IntoIterator
    let mut select_clause: DynamicSelectClause<TestDB, diesel_dynamic_schema::Table<&str>> =
        DynamicSelectClause::new();
    select_clause.add_field(name);

    // Extend
    let hair = users.column::<Untyped, _>("hair_color");
    let fields = vec![hair];
    select_clause.extend(fields);

    // IntoIterator
    assert_eq!(select_clause.len(), 2);
    assert!(!select_clause.is_empty());

    // Test DynamicRow ergonomics
    // Re-create query since select_clause was consumed
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");
    let mut select = DynamicSelectClause::new();
    select.add_fields(vec![name, hair_color]);

    let mut actual_data: Vec<DynamicRow<NamedField<MyDynamicValue>>> =
        users.select(select).load(connection).unwrap();

    let row = &mut actual_data[0];

    // IndexMut (usize)
    if let MyDynamicValue::String(ref mut s) = row[0].value {
        *s = "UpdatedName".to_string();
    }
    assert_eq!(
        row[0].value,
        MyDynamicValue::String("UpdatedName".to_string())
    );

    // IndexMut (str)
    if let MyDynamicValue::String(ref mut s) = row["hair_color"] {
        *s = "UpdatedHair".to_string();
    }
    assert_eq!(
        row["hair_color"],
        MyDynamicValue::String("UpdatedHair".to_string())
    );

    // Deref/DerefMut
    let field = &mut row[0];
    if let MyDynamicValue::String(ref mut s) = field.value {
        *s = "DerefUpdated".to_string();
    }
    // Check via deref
    assert_eq!(
        field.value,
        MyDynamicValue::String("DerefUpdated".to_string())
    );

    // Iter/IterMut on Row
    for field in row.iter_mut() {
        // field is &mut NamedField<MyDynamicValue>
        if let MyDynamicValue::String(ref mut s) = field.value {
            *s = format!("Iter_{}", s);
        }
    }
    assert_eq!(
        row[0].value,
        MyDynamicValue::String("Iter_DerefUpdated".to_string())
    );

    // IntoIterator for &mut DynamicRow
    for field in &mut *row {
        // field is &mut NamedField<MyDynamicValue>
        if let MyDynamicValue::String(ref mut s) = field.value {
            *s = format!("IntoIter_{}", s);
        }
    }
    assert_eq!(
        row[0].value,
        MyDynamicValue::String("IntoIter_Iter_DerefUpdated".to_string())
    );

    // From<Vec> and Into<Vec>
    // Construct simple row without names
    let raw_vec = vec![MyDynamicValue::Integer(1), MyDynamicValue::Integer(2)];
    let dyn_row: DynamicRow<MyDynamicValue> = raw_vec.into();
    let back_to_vec: Vec<MyDynamicValue> = dyn_row.into();
    assert_eq!(back_to_vec.len(), 2);
}

#[test]
fn dynamic_query() {
    let connection = &mut super::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let id = users.column::<Untyped, _>("id");
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");

    let mut select = DynamicSelectClause::new();

    select.add_field(id);
    select.add_field(name);
    select.add_field(hair_color);

    let actual_data: Vec<DynamicRow<NamedField<MyDynamicValue>>> =
        users.select(select).load(connection).unwrap();

    assert_eq!(
        actual_data[0]["name"],
        MyDynamicValue::String("Sean".into())
    );
    assert_eq!(
        actual_data[0][1],
        NamedField {
            name: "name".into(),
            value: MyDynamicValue::String("Sean".into())
        }
    );
    assert_eq!(
        actual_data[1]["name"],
        MyDynamicValue::String("Tess".into())
    );
    assert_eq!(
        actual_data[1][1],
        NamedField {
            name: "name".into(),
            value: MyDynamicValue::String("Tess".into())
        }
    );
    assert_eq!(
        actual_data[0]["hair_color"],
        MyDynamicValue::String("black".into())
    );
    assert_eq!(
        actual_data[0][2],
        NamedField {
            name: "hair_color".into(),
            value: MyDynamicValue::String("black".into())
        }
    );
    assert_eq!(
        actual_data[1]["hair_color"],
        MyDynamicValue::String("black".into())
    );
    assert_eq!(
        actual_data[1][2],
        NamedField {
            name: "hair_color".into(),
            value: MyDynamicValue::String("black".into())
        }
    );

    let mut select = DynamicSelectClause::new();

    select.add_field(id);
    select.add_field(name);
    select.add_field(hair_color);

    let actual_data: Vec<DynamicRow<MyDynamicValue>> =
        users.select(select).load(connection).unwrap();

    assert_eq!(actual_data[0][1], MyDynamicValue::String("Sean".into()));
    assert_eq!(actual_data[1][1], MyDynamicValue::String("Tess".into()));
    assert_eq!(actual_data[0][2], MyDynamicValue::String("black".into()));
    assert_eq!(actual_data[1][2], MyDynamicValue::String("black".into()));
}

#[test]
fn mixed_value_query() {
    use diesel::dsl::sql;

    let connection = &mut crate::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (id, name, hair_color) VALUES (42, 'Sean', 'black'), (43, 'Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let id = users.column::<Integer, _>("id");

    let (id, row) = users
        .select((id, sql::<Untyped>("name, hair_color")))
        .first::<(i32, DynamicRow<NamedField<MyDynamicValue>>)>(connection)
        .unwrap();

    assert_eq!(id, 42);
    assert_eq!(row["name"], MyDynamicValue::String("Sean".into()));
    assert_eq!(row["hair_color"], MyDynamicValue::String("black".into()));
}

#[test]
fn nullable_dynamic_value() {
    use diesel::dsl::sql;

    let connection = &mut crate::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (name, hair_color) VALUES ('Sean', 'dark'), ('Tess', NULL)")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");

    let result = users
        .select(sql::<Untyped>("hair_color"))
        .load::<DynamicRow<Option<MyDynamicValue>>>(connection)
        .unwrap();

    assert_eq!(result[0][0], Some(MyDynamicValue::String("dark".into())));
    assert_eq!(result[1][0], None);

    let result = users
        .select(sql::<Untyped>("hair_color"))
        .load::<DynamicRow<NamedField<Option<MyDynamicValue>>>>(connection)
        .unwrap();

    assert_eq!(
        result[0]["hair_color"],
        Some(MyDynamicValue::String("dark".into()))
    );
    assert_eq!(result[1]["hair_color"], None);
}

#[test]
fn dynamic_value_row_shapes() {
    let connection = &mut crate::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (id, name) VALUES (7, 'Sean')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let id = users.column::<Untyped, _>("id");
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");

    let mut select = DynamicSelectClause::new();
    select.add_field(id);
    select.add_field(name);
    select.add_field(hair_color);
    let rows: Vec<DynamicRow<DynamicValue>> = users.select(select).load(connection).unwrap();
    assert_eq!(rows[0][0], DynamicValue::Int(7));
    assert_eq!(rows[0][1], DynamicValue::Text("Sean".into()));
    assert_eq!(rows[0][2], DynamicValue::Null);

    let id = users.column::<Untyped, _>("id");
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");
    let mut select = DynamicSelectClause::new();
    select.add_field(id);
    select.add_field(name);
    select.add_field(hair_color);
    let rows: Vec<DynamicRow<NamedField<DynamicValue>>> =
        users.select(select).load(connection).unwrap();
    assert_eq!(rows[0]["name"], DynamicValue::Text("Sean".into()));
    assert_eq!(rows[0]["hair_color"], DynamicValue::Null);
}

#[test]
fn dynamic_value_core_scalars() {
    let connection = &mut crate::establish_connection();
    let ddl = if cfg!(feature = "postgres") {
        "CREATE TABLE dv_core (i INTEGER, big BIGINT, f DOUBLE PRECISION, t TEXT, b BYTEA)"
    } else if cfg!(feature = "sqlite") {
        "CREATE TABLE dv_core (i INTEGER, big BIGINT, f DOUBLE, t TEXT, b BLOB)"
    } else {
        "CREATE TEMPORARY TABLE dv_core (i INTEGER, big BIGINT, f DOUBLE, t TEXT, b BLOB)"
    };
    sql_query(ddl).execute(connection).unwrap();
    let blob = if cfg!(feature = "postgres") {
        "'\\x0102'"
    } else {
        "X'0102'"
    };
    sql_query(format!(
        "INSERT INTO dv_core (i, big, f, t, b) VALUES (42, 9000000000, 1.5, 'hi', {blob})"
    ))
    .execute(connection)
    .unwrap();

    let rows = sql_query("SELECT i, big, f, t, b FROM dv_core")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row[0], DynamicValue::Int(42));
    assert_eq!(row[1], DynamicValue::Int(9_000_000_000));
    assert_eq!(row[2], DynamicValue::Float(1.5));
    assert_eq!(row[3], DynamicValue::Text("hi".into()));
    assert_eq!(row[4], DynamicValue::Bytes(vec![1, 2]));
}

#[test]
fn dynamic_value_boolean_mapping() {
    let connection = &mut crate::establish_connection();
    let ddl = if cfg!(any(feature = "mysql", feature = "mariadb")) {
        "CREATE TEMPORARY TABLE dv_bool (val BOOLEAN)"
    } else {
        "CREATE TABLE dv_bool (val BOOLEAN)"
    };
    sql_query(ddl).execute(connection).unwrap();
    sql_query("INSERT INTO dv_bool (val) VALUES (TRUE)")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT val FROM dv_bool")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    let expected = if cfg!(feature = "postgres") {
        DynamicValue::Bool(true)
    } else {
        DynamicValue::Int(1)
    };
    assert_eq!(rows[0][0], expected);
}

#[cfg(all(
    feature = "chrono",
    any(feature = "postgres", feature = "mysql", feature = "mariadb")
))]
#[test]
fn dynamic_value_time_mapping() {
    let connection = &mut crate::establish_connection();

    #[cfg(feature = "postgres")]
    {
        sql_query("CREATE TABLE dv_time (val TIME)")
            .execute(connection)
            .unwrap();
        sql_query("INSERT INTO dv_time (val) VALUES ('12:34:56')")
            .execute(connection)
            .unwrap();
        let rows = sql_query("SELECT val FROM dv_time")
            .load::<DynamicRow<DynamicValue>>(connection)
            .unwrap();
        assert_eq!(
            rows[0][0],
            DynamicValue::Time(chrono::NaiveTime::from_hms_opt(12, 34, 56).unwrap())
        );
    }

    #[cfg(any(feature = "mysql", feature = "mariadb"))]
    {
        sql_query("CREATE TEMPORARY TABLE dv_time (a TIME, b TIME)")
            .execute(connection)
            .unwrap();
        sql_query("INSERT INTO dv_time (a, b) VALUES ('25:00:00', '12:34:56')")
            .execute(connection)
            .unwrap();
        let rows = sql_query("SELECT a, b FROM dv_time")
            .load::<DynamicRow<DynamicValue>>(connection)
            .unwrap();
        assert_eq!(
            rows[0][0],
            DynamicValue::Duration(chrono::Duration::seconds(90000))
        );
        assert_eq!(
            rows[0][1],
            DynamicValue::Duration(chrono::Duration::seconds(45296))
        );

        // Diesel rejects negative `MysqlTime` before this decoder runs.
        sql_query("CREATE TEMPORARY TABLE dv_time_neg (t TIME)")
            .execute(connection)
            .unwrap();
        sql_query("INSERT INTO dv_time_neg (t) VALUES ('-01:30:00')")
            .execute(connection)
            .unwrap();
        let negative =
            sql_query("SELECT t FROM dv_time_neg").load::<DynamicRow<DynamicValue>>(connection);
        match negative {
            Err(diesel::result::Error::DeserializationError(e)) => {
                assert!(e
                    .to_string()
                    .contains("Negative dates/times are not yet supported"));
            }
            other => panic!(
                "expected a deserialization error for a negative TIME, got {:?}",
                other
            ),
        }
    }
}

#[cfg(all(
    feature = "chrono",
    any(feature = "postgres", feature = "mysql", feature = "mariadb")
))]
#[test]
fn dynamic_value_date_and_timestamp() {
    let connection = &mut crate::establish_connection();
    let ddl = if cfg!(feature = "postgres") {
        "CREATE TABLE dv_ts (ts TIMESTAMP, d DATE)"
    } else {
        "CREATE TEMPORARY TABLE dv_ts (ts DATETIME, d DATE)"
    };
    sql_query(ddl).execute(connection).unwrap();
    sql_query("INSERT INTO dv_ts (ts, d) VALUES ('2020-01-02 03:04:05', '2020-01-02')")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT ts, d FROM dv_ts")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    assert_eq!(
        rows[0][0],
        DynamicValue::Timestamp(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 5)
                .unwrap()
        )
    );
    assert_eq!(
        rows[0][1],
        DynamicValue::Date(chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap())
    );
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[test]
fn dynamic_value_unsigned_above_i64_max() {
    let connection = &mut crate::establish_connection();
    sql_query("CREATE TEMPORARY TABLE dv_uns (a BIGINT UNSIGNED, b INT UNSIGNED)")
        .execute(connection)
        .unwrap();
    sql_query("INSERT INTO dv_uns (a, b) VALUES (18446744073709551615, 4294967295)")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT a, b FROM dv_uns")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    assert_eq!(rows[0][0], DynamicValue::UInt(u64::MAX));
    assert_eq!(rows[0][1], DynamicValue::UInt(4_294_967_295));
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[test]
fn dynamic_value_json_is_not_a_json_variant_on_mysql_like() {
    let connection = &mut crate::establish_connection();
    sql_query("CREATE TEMPORARY TABLE dv_mjson (j JSON)")
        .execute(connection)
        .unwrap();
    sql_query("INSERT INTO dv_mjson (j) VALUES ('{\"a\": 1}')")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT j FROM dv_mjson")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    let payload = match &rows[0][0] {
        DynamicValue::Text(s) => s.clone().into_bytes(),
        DynamicValue::Bytes(b) => b.clone(),
        other => panic!(
            "expected Text or Bytes for a MySQL-like JSON value, got {:?}",
            other
        ),
    };
    assert!(String::from_utf8_lossy(&payload).contains("\"a\""));
}

#[cfg(all(
    feature = "numeric",
    any(feature = "postgres", feature = "mysql", feature = "mariadb")
))]
#[test]
fn dynamic_value_numeric() {
    let connection = &mut crate::establish_connection();
    let ddl = if cfg!(feature = "postgres") {
        "CREATE TABLE dv_num (n NUMERIC(10, 2))"
    } else {
        "CREATE TEMPORARY TABLE dv_num (n NUMERIC(10, 2))"
    };
    sql_query(ddl).execute(connection).unwrap();
    sql_query("INSERT INTO dv_num (n) VALUES (123.45)")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT n FROM dv_num")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    assert_eq!(rows[0][0], DynamicValue::Numeric("123.45".parse().unwrap()));
}

#[cfg(all(feature = "uuid", feature = "postgres"))]
#[test]
fn dynamic_value_uuid() {
    let connection = &mut crate::establish_connection();
    sql_query("CREATE TABLE dv_uuid (u UUID)")
        .execute(connection)
        .unwrap();
    sql_query("INSERT INTO dv_uuid (u) VALUES ('11111111-1111-1111-1111-111111111111')")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT u FROM dv_uuid")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    assert_eq!(
        rows[0][0],
        DynamicValue::Uuid("11111111-1111-1111-1111-111111111111".parse().unwrap())
    );
}

#[cfg(all(feature = "serde_json", feature = "postgres"))]
#[test]
fn dynamic_value_json_and_jsonb() {
    let connection = &mut crate::establish_connection();
    sql_query("CREATE TABLE dv_json (j JSON, jb JSONB)")
        .execute(connection)
        .unwrap();
    sql_query("INSERT INTO dv_json (j, jb) VALUES ('{\"a\": 1}', '{\"b\": 2}')")
        .execute(connection)
        .unwrap();

    let rows = sql_query("SELECT j, jb FROM dv_json")
        .load::<DynamicRow<DynamicValue>>(connection)
        .unwrap();
    assert_eq!(rows[0][0], DynamicValue::Json(serde_json::json!({"a": 1})));
    assert_eq!(rows[0][1], DynamicValue::Jsonb(serde_json::json!({"b": 2})));
}

#[cfg(feature = "postgres")]
#[test]
fn dynamic_value_rejects_unsupported_pg_oid() {
    let connection = &mut crate::establish_connection();
    let result = sql_query("SELECT ARRAY[1, 2] AS a").load::<DynamicRow<DynamicValue>>(connection);
    assert!(result.is_err());
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[test]
fn dynamic_value_rejects_unsupported_mysql_like_tag() {
    let connection = &mut crate::establish_connection();
    sql_query("CREATE TEMPORARY TABLE dv_bit (b BIT(3))")
        .execute(connection)
        .unwrap();
    sql_query("INSERT INTO dv_bit (b) VALUES (b'101')")
        .execute(connection)
        .unwrap();

    let result = sql_query("SELECT b FROM dv_bit").load::<DynamicRow<DynamicValue>>(connection);
    assert!(result.is_err());
}

#[cfg(all(feature = "postgres", not(feature = "numeric")))]
#[test]
fn dynamic_value_reports_disabled_feature() {
    let connection = &mut crate::establish_connection();
    sql_query("CREATE TABLE dv_fd (n NUMERIC(10, 2))")
        .execute(connection)
        .unwrap();
    sql_query("INSERT INTO dv_fd (n) VALUES (1.00)")
        .execute(connection)
        .unwrap();

    let result = sql_query("SELECT n FROM dv_fd").load::<DynamicRow<DynamicValue>>(connection);
    assert!(result.is_err());
}
