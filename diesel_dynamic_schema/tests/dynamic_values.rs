use diesel::deserialize::*;
use diesel::prelude::*;
use diesel::sql_types::*;
use diesel_dynamic_schema::dynamic_value::*;
use diesel_dynamic_schema::{DynamicOutputClause, DynamicSchemaError};

#[derive(PartialEq, Debug)]
enum MyDynamicValue {
    String(String),
    Integer(i32),
}

#[cfg(feature = "postgres")]
impl FromSql<Any, diesel::pg::Pg> for MyDynamicValue {
    fn from_sql(value: diesel::pg::PgValue<'_>) -> Result<Self> {
        use core::num::NonZeroU32;
        use diesel::pg::Pg;

        const VARCHAR_OID: NonZeroU32 = NonZeroU32::new(1043).unwrap();
        const TEXT_OID: NonZeroU32 = NonZeroU32::new(25).unwrap();
        const INTEGER_OID: NonZeroU32 = NonZeroU32::new(23).unwrap();

        match value.get_oid() {
            VARCHAR_OID | TEXT_OID => {
                <String as FromSql<Text, Pg>>::from_sql(value).map(MyDynamicValue::String)
            }
            INTEGER_OID => {
                <i32 as FromSql<Integer, Pg>>::from_sql(value).map(MyDynamicValue::Integer)
            }
            oid => Err(format!("Unknown type: {oid}").into()),
        }
    }
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
impl FromSql<Any, diesel::sqlite::Sqlite> for MyDynamicValue {
    fn from_sql(value: diesel::sqlite::SqliteValue<'_, '_, '_>) -> Result<Self> {
        use diesel::sqlite::{Sqlite, SqliteType};

        match value.value_type() {
            Some(SqliteType::Text) => {
                <String as FromSql<Text, Sqlite>>::from_sql(value).map(MyDynamicValue::String)
            }
            Some(SqliteType::Long) => {
                <i32 as FromSql<Integer, Sqlite>>::from_sql(value).map(MyDynamicValue::Integer)
            }
            _ => Err("unknown data type".into()),
        }
    }
}

#[cfg(feature = "mysql")]
impl FromSql<Any, diesel::mysql::Mysql> for MyDynamicValue {
    fn from_sql(value: diesel::mysql::MysqlValue<'_>) -> Result<Self> {
        use diesel::mysql::{Mysql, MysqlType};

        match value.value_type() {
            MysqlType::String => {
                <String as FromSql<Text, Mysql>>::from_sql(value).map(MyDynamicValue::String)
            }
            MysqlType::Long => {
                <i32 as FromSql<Integer, Mysql>>::from_sql(value).map(MyDynamicValue::Integer)
            }
            tag => Err(format!("unknown data type: {tag:?}").into()),
        }
    }
}

#[cfg(feature = "mariadb")]
impl FromSql<Any, diesel::mariadb::Mariadb> for MyDynamicValue {
    fn from_sql(value: diesel::mariadb::MariadbValue<'_>) -> Result<Self> {
        use diesel::mariadb::{Mariadb, MariadbType};

        match value.value_type() {
            MariadbType::String => {
                <String as FromSql<Text, Mariadb>>::from_sql(value).map(MyDynamicValue::String)
            }
            MariadbType::Long => {
                <i32 as FromSql<Integer, Mariadb>>::from_sql(value).map(MyDynamicValue::Integer)
            }
            tag => Err(format!("unknown data type: {tag:?}").into()),
        }
    }
}

#[cfg(feature = "postgres")]
type TestDB = diesel::pg::Pg;
#[cfg(all(not(feature = "postgres"), feature = "mysql"))]
type TestDB = diesel::mysql::Mysql;
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "mariadb"))]
type TestDB = diesel::mariadb::Mariadb;
#[cfg(all(
    not(feature = "postgres"),
    not(feature = "mysql"),
    not(feature = "mariadb"),
    any(feature = "sqlite", feature = "sqlite-no-std")
))]
type TestDB = diesel::sqlite::Sqlite;

#[cfg(feature = "postgres")]
type TestConn = diesel::PgConnection;
#[cfg(all(not(feature = "postgres"), feature = "mysql"))]
type TestConn = diesel::MysqlConnection;
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "mariadb"))]
type TestConn = diesel::MariadbConnection;
#[cfg(all(
    not(feature = "postgres"),
    not(feature = "mysql"),
    not(feature = "mariadb"),
    any(feature = "sqlite", feature = "sqlite-no-std")
))]
type TestConn = diesel::SqliteConnection;

diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}
#[cfg(any(feature = "mysql", feature = "mariadb"))]
diesel::table! {
    wide_numbers (id) {
        id -> Integer,
        v -> Unsigned<BigInt>,
    }
}

#[cfg(all(any(feature = "sqlite", feature = "sqlite-no-std"), feature = "chrono"))]
diesel::table! {
    dv_sqlite_chrono_values (id) {
        id -> Integer,
        d -> Date,
    }
}

#[cfg(all(any(feature = "sqlite", feature = "sqlite-no-std"), feature = "time"))]
diesel::table! {
    dv_sqlite_time_values (id) {
        id -> Integer,
        d -> Date,
    }
}

#[cfg(all(
    any(feature = "sqlite", feature = "sqlite-no-std"),
    feature = "numeric"
))]
diesel::table! {
    dv_sqlite_numeric_values (id) {
        id -> Integer,
        n -> Numeric,
    }
}

#[cfg(all(
    any(feature = "sqlite", feature = "sqlite-no-std"),
    feature = "serde_json"
))]
diesel::table! {
    dv_sqlite_json_values (id) {
        id -> Integer,
        j -> Json,
    }
}

fn insert_users(conn: &mut TestConn) {
    diesel::insert_into(users::table)
        .values([
            (users::name.eq("Sean"), users::hair_color.eq(Some("black"))),
            (
                users::name.eq("Tess"),
                users::hair_color.eq::<Option<&str>>(None),
            ),
        ])
        .execute(conn)
        .unwrap();
}

fn dynamic_users() -> diesel_dynamic_schema::Table<&'static str> {
    diesel_dynamic_schema::table("users")
}

#[cfg(feature = "mariadb")]
fn mariadb_server_supports_update_returning(conn: &mut TestConn) -> bool {
    diesel::dsl::sql::<VarChar>("SELECT VERSION();")
        .get_result::<String>(conn)
        .expect("Failed to get MariaDB server version")
        .split('.')
        .next()
        .map(str::parse::<u32>)
        .expect("Failed to split MariaDB server version")
        .expect("Failed to parse MariaDB server version")
        >= 13
}

#[test]
fn dynamic_output_clause_keeps_legacy_from_sql_loading() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let id = users.column::<Untyped, _>("id");
    let name = users.column::<Untyped, _>("name");
    let mut output = DynamicOutputClause::new();
    output.add_field(id);
    output.add_field(name);

    let rows: Vec<DynamicRow<NamedField<MyDynamicValue>>> =
        users.select(output).load(conn).unwrap();

    assert!(matches!(rows[0]["id"], MyDynamicValue::Integer(_)));
    assert_eq!(rows[0]["name"], MyDynamicValue::String("Sean".into()));
}

#[test]
fn dynamic_output_clause_supports_builder_and_extend() {
    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let hair_color = users.column::<Nullable<Text>, _>("hair_color");
    let mut output: DynamicOutputClause<TestDB, diesel_dynamic_schema::Table<&str>> =
        DynamicOutputClause::new().field(name);

    assert_eq!(output.len(), 1);
    assert!(!output.is_empty());

    output.extend([hair_color]);
    assert_eq!(output.len(), 2);
}

#[test]
fn named_loading_uses_direct_column_metadata() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");
    let hair_color = users.column::<Nullable<Text>, _>("hair_color");
    let output = DynamicOutputClause::new()
        .field(id)
        .field(name)
        .field(hair_color);

    let rows = users.select(output).load_dynamic(conn).unwrap();

    assert!(matches!(rows[0]["id"], DynamicValue::Integer(_)));
    assert_eq!(rows[0]["name"], DynamicValue::Text("Sean".into()));
    assert_eq!(rows[0]["hair_color"], DynamicValue::Text("black".into()));
    assert_eq!(rows[1]["hair_color"], DynamicValue::Null);
}

#[test]
fn positional_loading_accepts_untyped_columns() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let name = users.column::<Untyped, _>("name");
    let hair_color = users.column::<Untyped, _>("hair_color");
    let output = DynamicOutputClause::new().field(name).field(hair_color);

    let rows = users.select(output).load_dynamic_values(conn).unwrap();

    assert_eq!(rows[0][0], DynamicValue::Text("Sean".into()));
    assert_eq!(rows[1][1], DynamicValue::Null);
}

#[test]
fn tuple_output_metadata_matches_rendered_order() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");
    let output = DynamicOutputClause::new().field((id, name));

    let rows = users.select(output).load_dynamic_values(conn).unwrap();

    assert!(matches!(rows[0][0], DynamicValue::Integer(_)));
    assert_eq!(rows[0][1], DynamicValue::Text("Sean".into()));
}

#[derive(Debug, Clone, Copy)]
struct NoMetadataSql;

impl Expression for NoMetadataSql {
    type SqlType = Integer;
}

impl<DB> diesel::query_builder::QueryFragment<DB> for NoMetadataSql
where
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::QueryResult<()> {
        pass.push_sql("1");
        Ok(())
    }

    fn collect_output_metadata<'b>(
        &'b self,
        _out: &mut Vec<diesel::query_builder::OutputFieldMetadata<'b>>,
    ) -> diesel::QueryResult<()> {
        Ok(())
    }
}

impl<QS> diesel::AppearsOnTable<QS> for NoMetadataSql {}

impl<QS> diesel::SelectableExpression<QS> for NoMetadataSql {}

impl diesel::expression::ValidGrouping<()> for NoMetadataSql {
    type IsAggregate = diesel::expression::is_aggregate::No;
}

#[test]
fn output_field_count_mismatch_is_reported() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let output = DynamicOutputClause::new().field(NoMetadataSql);
    let err = users.select(output).load_dynamic_values(conn).unwrap_err();

    match err {
        diesel::result::Error::DeserializationError(err) => {
            match err.downcast_ref::<DynamicSchemaError>() {
                Some(DynamicSchemaError::OutputFieldCountMismatch { metadata, row }) => {
                    assert_eq!((*metadata, *row), (0, 1));
                }
                other => panic!("unexpected dynamic-schema error: {:?}", other),
            }
        }
        other => panic!("unexpected Diesel error: {:?}", other),
    }
}

#[test]
fn unnamed_field_error_is_matchable() {
    assert_eq!(
        DynamicSchemaError::UnnamedField.to_string(),
        "dynamic output field has no name"
    );
}

#[test]
fn backend_dynamic_value_alias_stays_generic() {
    fn is_null_backend_value<DB, E>(value: &BackendDynamicValue<DB, E>) -> bool
    where
        DB: DynamicValueBackend,
    {
        matches!(value, DynamicValue::Null)
    }

    let value: DefaultDynamicValue<TestDB> = DynamicValue::Null;
    assert!(is_null_backend_value::<TestDB, core::convert::Infallible>(
        &value
    ));
}

#[test]
fn iterator_and_single_row_loading_match_vector_loading() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let output = DynamicOutputClause::new().field(name);
    let query = users.select(output);

    let vector_rows = query.load_dynamic(conn).unwrap();
    let iter_first = {
        let mut iter_rows = query.load_dynamic_iter(conn).unwrap();
        iter_rows.next().unwrap().unwrap()
    };
    let first_row = query.get_dynamic_result(conn).unwrap();
    let result_rows = query.get_dynamic_results(conn).unwrap();

    assert_eq!(iter_first["name"], vector_rows[0]["name"]);
    assert_eq!(first_row["name"], vector_rows[0]["name"]);
    assert_eq!(result_rows[1]["name"], DynamicValue::Text("Tess".into()));

    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let output = DynamicOutputClause::new().field(name);
    let positional_query = users.select(output);
    let mut positional = positional_query.load_dynamic_values_iter(conn).unwrap();

    assert_eq!(
        positional.next().unwrap().unwrap()[0],
        DynamicValue::Text("Sean".into())
    );
}

struct PrefixExtension(&'static str);

impl DynamicValueExtension<TestDB> for PrefixExtension {
    type Value = String;

    fn claims(&self, context: &DynamicDecodeContext<'_, TestDB>) -> bool {
        context
            .origin()
            .map(|origin| origin.column == "name")
            .unwrap_or(false)
    }

    fn decode(
        &self,
        _context: &DynamicDecodeContext<'_, TestDB>,
        value: <TestDB as diesel::backend::Backend>::RawValue<'_>,
    ) -> Result<Self::Value> {
        <String as FromSql<Text, TestDB>>::from_sql(value).map(|value| format!("{}{value}", self.0))
    }
}

struct ChainExtension {
    first: PrefixExtension,
    second: PrefixExtension,
}

impl DynamicValueExtension<TestDB> for ChainExtension {
    type Value = String;

    fn claims(&self, context: &DynamicDecodeContext<'_, TestDB>) -> bool {
        self.first.claims(context) || self.second.claims(context)
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, TestDB>,
        value: <TestDB as diesel::backend::Backend>::RawValue<'_>,
    ) -> Result<Self::Value> {
        if self.first.claims(context) {
            self.first.decode(context, value)
        } else {
            self.second.decode(context, value)
        }
    }
}

#[test]
fn user_composed_extensions_choose_first_claim() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let output = DynamicOutputClause::new().field(name);
    let chain = ChainExtension {
        first: PrefixExtension("first:"),
        second: PrefixExtension("second:"),
    };

    let rows = users
        .select(output)
        .load_dynamic_with(conn, &chain)
        .unwrap();

    assert_eq!(rows[0]["name"], DynamicValue::Custom("first:Sean".into()));
}

#[test]
fn query_reuses_borrowed_extensions() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let output = DynamicOutputClause::new().field(name);
    let query = users.select(output);
    let left = PrefixExtension("left:");
    let right = PrefixExtension("right:");

    let left_rows = query.load_dynamic_with(conn, &left).unwrap();
    let right_rows = query.load_dynamic_with(conn, &right).unwrap();

    assert_eq!(
        left_rows[0]["name"],
        DynamicValue::Custom("left:Sean".into())
    );
    assert_eq!(
        right_rows[0]["name"],
        DynamicValue::Custom("right:Sean".into())
    );
}

#[test]
fn boxed_select_preserves_dynamic_output_metadata() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let hair_color = users.column::<Nullable<Text>, _>("hair_color");
    let output = DynamicOutputClause::new().field(name).field(hair_color);
    let rows = users
        .select(output)
        .into_boxed::<TestDB>()
        .load_dynamic_with(conn, &PrefixExtension("boxed:"))
        .unwrap();

    assert_eq!(rows[0]["name"], DynamicValue::Custom("boxed:Sean".into()));
    assert_eq!(rows[0]["hair_color"], DynamicValue::Text("black".into()));

    let users = dynamic_users();
    let name = users.column::<Text, _>("name");
    let hair_color = users.column::<Nullable<Text>, _>("hair_color");
    let query = users
        .select((name, hair_color))
        .into_boxed_clone::<TestDB>();
    let rows = query
        .clone()
        .load_dynamic_with(conn, &PrefixExtension("clone:"))
        .unwrap();

    assert_eq!(rows[0]["name"], DynamicValue::Custom("clone:Sean".into()));
    assert_eq!(rows[0]["hair_color"], DynamicValue::Text("black".into()));
}

#[cfg(any(
    feature = "postgres",
    feature = "mariadb",
    feature = "returning_clauses_for_sqlite_3_35"
))]
#[test]
fn existing_returning_insert_uses_dynamic_output_clause() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);

    let output = DynamicOutputClause::new()
        .field(users::id)
        .field(users::name);
    let row = diesel::insert_into(users::table)
        .values((users::name.eq("Sean"), users::hair_color.eq(Some("black"))))
        .returning(output)
        .get_dynamic_result(conn)
        .unwrap();

    assert!(matches!(row["id"], DynamicValue::Integer(_)));
    assert_eq!(row["name"], DynamicValue::Text("Sean".into()));
}

#[cfg(any(
    feature = "postgres",
    feature = "mariadb",
    feature = "returning_clauses_for_sqlite_3_35"
))]
#[test]
fn existing_returning_update_uses_dynamic_output_clause() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    #[cfg(feature = "mariadb")]
    if !mariadb_server_supports_update_returning(conn) {
        return;
    }

    let output = DynamicOutputClause::new()
        .field(users::id)
        .field(users::hair_color);
    let row = diesel::update(users::table.filter(users::name.eq("Sean")))
        .set(users::hair_color.eq(Some("brown")))
        .returning(output)
        .get_dynamic_result(conn)
        .unwrap();

    assert!(matches!(row["id"], DynamicValue::Integer(_)));
    assert_eq!(row["hair_color"], DynamicValue::Text("brown".into()));
}

#[cfg(any(
    feature = "postgres",
    feature = "mariadb",
    feature = "returning_clauses_for_sqlite_3_35"
))]
#[test]
fn existing_returning_delete_uses_dynamic_output_clause() {
    let conn = &mut super::establish_connection();
    crate::create_user_table(conn);
    insert_users(conn);

    let output = DynamicOutputClause::new()
        .field(users::id)
        .field(users::name);
    let row = diesel::delete(users::table.filter(users::name.eq("Sean")))
        .returning(output)
        .get_dynamic_result(conn)
        .unwrap();

    assert!(matches!(row["id"], DynamicValue::Integer(_)));
    assert_eq!(row["name"], DynamicValue::Text("Sean".into()));
}

#[cfg(all(
    feature = "postgres",
    any(
        feature = "chrono",
        feature = "time",
        feature = "numeric",
        feature = "uuid",
        feature = "serde_json",
        feature = "network-address",
        feature = "ipnet-address"
    )
))]
fn pg_scalar_value<Ext>(
    conn: &mut diesel::PgConnection,
    expression: &'static str,
    extension: &Ext,
) -> BackendDynamicValue<diesel::pg::Pg, Ext::Value>
where
    Ext: DynamicValueExtension<diesel::pg::Pg>,
{
    use diesel::dsl::sql;

    crate::create_user_table(conn);
    insert_users(conn);
    let output = DynamicOutputClause::new().field(sql::<Untyped>(expression));
    let rows = dynamic_users()
        .select(output)
        .load_dynamic_values_with(conn, extension)
        .unwrap();
    rows.into_iter().next().unwrap().into_iter().next().unwrap()
}

#[cfg(all(feature = "postgres", feature = "chrono"))]
#[test]
fn postgres_chrono_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(conn, "'2000-01-02'::date", &ChronoExtension);

    assert_eq!(
        value,
        DynamicValue::Custom(ChronoValue::Date(
            chrono::NaiveDate::from_ymd_opt(2000, 1, 2).unwrap()
        ))
    );
}

#[cfg(all(feature = "postgres", feature = "time"))]
#[test]
fn postgres_time_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(conn, "'2000-01-02'::date", &TimeExtension);

    assert_eq!(
        value,
        DynamicValue::Custom(TimeValue::Date(
            time::Date::from_calendar_date(2000, time::Month::January, 2).unwrap()
        ))
    );
}

#[cfg(all(feature = "postgres", feature = "numeric"))]
#[test]
fn postgres_bigdecimal_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(conn, "12.5::numeric", &BigDecimalExtension);

    assert_eq!(
        value,
        DynamicValue::Custom("12.5".parse::<bigdecimal::BigDecimal>().unwrap())
    );
}

#[cfg(all(feature = "postgres", feature = "uuid"))]
#[test]
fn postgres_uuid_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(
        conn,
        "'67e55044-10b1-426f-9247-bb680e5fe0c8'::uuid",
        &UuidExtension,
    );

    assert_eq!(
        value,
        DynamicValue::Custom("67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap())
    );
}

#[cfg(all(feature = "postgres", feature = "serde_json"))]
#[test]
fn postgres_json_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(conn, "'{\"a\":1}'::jsonb", &SerdeJsonExtension);

    assert_eq!(value, DynamicValue::Custom(serde_json::json!({"a": 1})));
}

#[cfg(all(feature = "postgres", feature = "network-address"))]
#[test]
fn postgres_ipnetwork_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(conn, "'10.1.0.0/24'::inet", &IpNetworkExtension);

    assert_eq!(
        value,
        DynamicValue::Custom("10.1.0.0/24".parse::<ipnetwork::IpNetwork>().unwrap())
    );
}

#[cfg(all(feature = "postgres", feature = "ipnet-address"))]
#[test]
fn postgres_ipnet_extension_claims_backend_tags() {
    let conn = &mut super::establish_connection();
    let value = pg_scalar_value(conn, "'10.1.0.0/24'::inet", &IpNetExtension);

    assert_eq!(
        value,
        DynamicValue::Custom("10.1.0.0/24".parse::<ipnet::IpNet>().unwrap())
    );
}

#[cfg(feature = "postgres")]
fn pg_untyped_scalar(
    conn: &mut diesel::PgConnection,
    expression: &'static str,
) -> DefaultDynamicValue<diesel::pg::Pg> {
    use diesel::dsl::sql;

    let output = DynamicOutputClause::new().field(sql::<Untyped>(expression));
    let rows = diesel::select(output).load_dynamic_values(conn).unwrap();
    rows.into_iter().next().unwrap().into_iter().next().unwrap()
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_crate_free_backend_variants_are_structured() {
    use diesel_dynamic_schema::dynamic_value::pg::PgBackendValue;

    let conn = &mut super::establish_connection();

    assert!(matches!(
        pg_untyped_scalar(conn, "'2000-01-02'::date"),
        DynamicValue::Backend(PgBackendValue::Date(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'01:02:03'::time"),
        DynamicValue::Backend(PgBackendValue::Time(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'2000-01-02 03:04:05'::timestamp"),
        DynamicValue::Backend(PgBackendValue::Timestamp(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'2000-01-02 03:04:05+00'::timestamptz"),
        DynamicValue::Backend(PgBackendValue::Timestamptz(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'1 day'::interval"),
        DynamicValue::Backend(PgBackendValue::Interval(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "12.5::numeric"),
        DynamicValue::Backend(PgBackendValue::Numeric(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "12.34::money"),
        DynamicValue::Backend(PgBackendValue::Money(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'67e55044-10b1-426f-9247-bb680e5fe0c8'::uuid"),
        DynamicValue::Backend(PgBackendValue::Uuid(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'{\"a\":1}'::json"),
        DynamicValue::Backend(PgBackendValue::Json(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'{\"a\":1}'::jsonb"),
        DynamicValue::Backend(PgBackendValue::Jsonb(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'08:00:2b:01:02:03'::macaddr"),
        DynamicValue::Backend(PgBackendValue::MacAddr(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'08:00:2b:01:02:03:04:05'::macaddr8"),
        DynamicValue::Backend(PgBackendValue::MacAddr8(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "42::oid"),
        DynamicValue::Backend(PgBackendValue::Oid(42))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'a'::\"char\""),
        DynamicValue::Backend(PgBackendValue::CChar(_))
    ));
    assert!(matches!(
        pg_untyped_scalar(conn, "'0/16B6C50'::pg_lsn"),
        DynamicValue::Backend(PgBackendValue::PgLsn(_))
    ));
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[test]
fn mysql_like_unsigned_bigint_uses_unsigned_value() {
    let conn = &mut super::establish_connection();
    // MySQL table DDL has no typed query DSL.
    diesel::sql_query("CREATE TEMPORARY TABLE wide_numbers (id INTEGER PRIMARY KEY AUTO_INCREMENT, v BIGINT UNSIGNED NOT NULL)")
        .execute(conn)
        .unwrap();
    diesel::insert_into(wide_numbers::table)
        .values(wide_numbers::v.eq(9_223_372_036_854_775_808_u64))
        .execute(conn)
        .unwrap();

    let table = diesel_dynamic_schema::table("wide_numbers");
    let typed = table.column::<Unsigned<BigInt>, _>("v");
    let untyped = table.column::<Untyped, _>("v");
    let output = DynamicOutputClause::new().field(typed).field(untyped);
    let rows = table.select(output).load_dynamic_values(conn).unwrap();

    assert_eq!(
        rows[0][0],
        DynamicValue::Unsigned(9_223_372_036_854_775_808)
    );
    assert_eq!(
        rows[0][1],
        DynamicValue::Unsigned(9_223_372_036_854_775_808)
    );
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[test]
fn mysql_like_current_tags_are_structured() {
    use diesel_dynamic_schema::dynamic_value::mysql_like::MysqlLikeBackendValue;

    let conn = &mut super::establish_connection();
    // MySQL table DDL has no typed query DSL.
    diesel::sql_query(
        "CREATE TEMPORARY TABLE dv_mysql_like_tags (
            v_tiny TINYINT NOT NULL,
            v_utiny TINYINT UNSIGNED NOT NULL,
            v_short SMALLINT NOT NULL,
            v_ushort SMALLINT UNSIGNED NOT NULL,
            v_long INTEGER NOT NULL,
            v_ulong INTEGER UNSIGNED NOT NULL,
            v_longlong BIGINT NOT NULL,
            v_ulonglong BIGINT UNSIGNED NOT NULL,
            v_float FLOAT NOT NULL,
            v_double DOUBLE NOT NULL,
            v_numeric DECIMAL(10,2) NOT NULL,
            v_date DATE NOT NULL,
            v_time TIME(6) NOT NULL,
            v_datetime DATETIME NOT NULL,
            v_timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            v_string TEXT NOT NULL,
            v_blob BLOB NOT NULL,
            v_bit BIT(4) NOT NULL,
            v_set SET('a','b') NOT NULL,
            v_enum ENUM('x','y') NOT NULL
        )",
    )
    .execute(conn)
    .unwrap();
    // MySQL tag fixture uses server literals to force wire tags Diesel cannot write generically.
    diesel::sql_query(
        "INSERT INTO dv_mysql_like_tags (
            v_tiny, v_utiny, v_short, v_ushort, v_long, v_ulong, v_longlong, v_ulonglong,
            v_float, v_double, v_numeric, v_date, v_time, v_datetime, v_timestamp,
            v_string, v_blob, v_bit, v_set, v_enum
        ) VALUES (
            -1, 2, -3, 4, -5, 6, -7, 9223372036854775808,
            1.5, 2.5, 12.50, '2000-01-02', '-01:02:03.123456', '2000-01-02 03:04:05',
            '2000-01-02 03:04:05', 'text', X'0102', b'1010', 'a,b', 'y'
        )",
    )
    .execute(conn)
    .unwrap();

    let table = diesel_dynamic_schema::table("dv_mysql_like_tags");
    let mut output: DynamicOutputClause<TestDB, diesel_dynamic_schema::Table<&str>> =
        DynamicOutputClause::new();
    for column in [
        "v_tiny",
        "v_utiny",
        "v_short",
        "v_ushort",
        "v_long",
        "v_ulong",
        "v_longlong",
        "v_ulonglong",
        "v_float",
        "v_double",
        "v_numeric",
        "v_date",
        "v_time",
        "v_datetime",
        "v_timestamp",
        "v_string",
        "v_blob",
        "v_bit",
        "v_set",
        "v_enum",
    ] {
        output.add_field(table.column::<Untyped, _>(column));
    }

    let rows = table.select(output).load_dynamic_values(conn).unwrap();
    let row = &rows[0];

    assert!(matches!(row[0], DynamicValue::Integer(-1)));
    assert!(matches!(row[1], DynamicValue::Unsigned(2)));
    assert!(matches!(row[2], DynamicValue::Integer(-3)));
    assert!(matches!(row[3], DynamicValue::Unsigned(4)));
    assert!(matches!(row[4], DynamicValue::Integer(-5)));
    assert!(matches!(row[5], DynamicValue::Unsigned(6)));
    assert!(matches!(row[6], DynamicValue::Integer(-7)));
    assert!(matches!(
        row[7],
        DynamicValue::Unsigned(9_223_372_036_854_775_808)
    ));
    assert!(matches!(row[8], DynamicValue::Float(_)));
    assert!(matches!(row[9], DynamicValue::Float(2.5)));
    assert!(matches!(
        row[10],
        DynamicValue::Backend(MysqlLikeBackendValue::Numeric(_))
    ));
    assert!(matches!(
        row[11],
        DynamicValue::Backend(MysqlLikeBackendValue::Date(_))
    ));
    assert!(matches!(
        &row[12],
        DynamicValue::Backend(MysqlLikeBackendValue::Time(value))
            if value.neg
                && value.hour == 1
                && value.minute == 2
                && value.second == 3
                && value.second_part == 123_456
    ));
    assert!(matches!(
        row[13],
        DynamicValue::Backend(MysqlLikeBackendValue::DateTime(_))
    ));
    assert!(matches!(
        row[14],
        DynamicValue::Backend(MysqlLikeBackendValue::Timestamp(_))
    ));
    assert!(matches!(row[15], DynamicValue::Text(_)));
    assert!(matches!(row[16], DynamicValue::Bytes(_)));
    assert!(matches!(
        row[17],
        DynamicValue::Backend(MysqlLikeBackendValue::Bit(_))
    ));
    assert!(matches!(row[18], DynamicValue::Text(_)));
    assert!(matches!(row[19], DynamicValue::Text(_)));
}

#[cfg(feature = "postgres")]
#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "dv_mood"))]
struct DvMood;

#[cfg(feature = "postgres")]
fn pg_array_value<E>(
    value: &DynamicValue<diesel_dynamic_schema::dynamic_value::pg::PgBackendValue<E>, E>,
) -> &diesel_dynamic_schema::dynamic_value::pg::PgDynamicArray<E> {
    match value {
        DynamicValue::Backend(diesel_dynamic_schema::dynamic_value::pg::PgBackendValue::Array(
            array,
        )) => array,
        _ => panic!("expected PostgreSQL array"),
    }
}

#[cfg(feature = "postgres")]
fn pg_array_rows<Ext>(
    conn: &mut diesel::PgConnection,
    expression: impl diesel::query_builder::QueryFragment<diesel::pg::Pg>
        + diesel::SelectableExpression<diesel_dynamic_schema::Table<&'static str>>
        + diesel::expression::NonAggregate
        + Send
        + 'static,
    extension: &Ext,
) -> Vec<DynamicRow<BackendDynamicValue<diesel::pg::Pg, Ext::Value>>>
where
    Ext: DynamicValueExtension<diesel::pg::Pg>,
{
    crate::create_user_table(conn);
    insert_users(conn);
    let output = DynamicOutputClause::new().field(expression);
    dynamic_users()
        .select(output)
        .load_dynamic_values_with(conn, extension)
        .unwrap()
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_array_shape_and_values_are_dynamic() {
    use diesel::dsl::sql;
    use diesel::sql_types::{Array, Nullable};

    let conn = &mut super::establish_connection();
    let rows = pg_array_rows(
        conn,
        sql::<Array<Nullable<Integer>>>("ARRAY[1, NULL, 3]"),
        &NoDynamicValueExtension,
    );
    let array = pg_array_value(&rows[0][0]);

    assert_eq!(array.dimensions()[0].length, 3);
    assert_eq!(array.dimensions()[0].lower_bound, 1);
    assert_eq!(array.values()[0], DynamicValue::Integer(1));
    assert_eq!(array.values()[1], DynamicValue::Null);
    assert_eq!(array.get(&[3]), Some(&DynamicValue::Integer(3)));
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_name_array_uses_array_decoder_from_backend_tag() {
    let conn = &mut super::establish_connection();
    let rows = pg_array_rows(
        conn,
        diesel::dsl::sql::<Untyped>("ARRAY['alpha'::name, 'beta'::name]"),
        &NoDynamicValueExtension,
    );
    let array = pg_array_value(&rows[0][0]);

    assert_eq!(array.dimensions()[0].length, 2);
    assert_eq!(array.values()[0], DynamicValue::Text("alpha".into()));
    assert_eq!(array.values()[1], DynamicValue::Text("beta".into()));
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_array_multidimensional_and_lower_bounds_are_preserved() {
    use diesel::dsl::sql;
    use diesel::sql_types::Array;

    let conn = &mut super::establish_connection();
    let rows = pg_array_rows(
        conn,
        sql::<Array<Integer>>("'[2:3][4:5]={{1,2},{3,4}}'::integer[]"),
        &NoDynamicValueExtension,
    );
    let array = pg_array_value(&rows[0][0]);

    assert_eq!(array.dimensions()[0].lower_bound, 2);
    assert_eq!(array.dimensions()[1].lower_bound, 4);
    assert_eq!(array.get(&[2, 4]), Some(&DynamicValue::Integer(1)));
    assert_eq!(array.get(&[3, 5]), Some(&DynamicValue::Integer(4)));
    assert_eq!(array.get(&[1, 4]), None);
}

#[cfg(feature = "postgres")]
struct PgArrayElementExtension;

#[cfg(feature = "postgres")]
impl DynamicValueExtension<diesel::pg::Pg> for PgArrayElementExtension {
    type Value = String;

    fn claims(&self, context: &DynamicDecodeContext<'_, diesel::pg::Pg>) -> bool {
        context.pg_array_subscripts().is_some()
            && matches!(context.backend_tag(), diesel_dynamic_schema::dynamic_value::pg::PgTypeTag::Scalar(oid) if oid.get() == 23)
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, diesel::pg::Pg>,
        value: diesel::pg::PgValue<'_>,
    ) -> Result<Self::Value> {
        let value = <i32 as FromSql<Integer, diesel::pg::Pg>>::from_sql(value)?;
        let subscripts = context.pg_array_subscripts().unwrap();
        Ok(format!("{}:{}={value}", subscripts[0], subscripts[1]))
    }
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_array_elements_use_extensions_recursively() {
    use diesel::dsl::sql;
    use diesel::sql_types::Array;

    let conn = &mut super::establish_connection();
    let rows = pg_array_rows(
        conn,
        sql::<Array<Integer>>("ARRAY[[7,8],[9,10]]"),
        &PgArrayElementExtension,
    );
    let array = pg_array_value(&rows[0][0]);

    assert_eq!(array.values()[0], DynamicValue::Custom("1:1=7".into()));
    assert_eq!(array.values()[3], DynamicValue::Custom("2:2=10".into()));
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_unknown_array_elements_remain_opaque() {
    use diesel::sql_types::Array;

    let conn = &mut super::establish_connection();
    // PostgreSQL type DDL has no typed query DSL.
    diesel::sql_query("CREATE TYPE dv_mood AS ENUM ('happy')")
        .execute(conn)
        .unwrap();
    // Custom PostgreSQL array setup has no typed value writer in this test.
    diesel::sql_query("CREATE TABLE dv_unknown_arrays (moods dv_mood[] NOT NULL)")
        .execute(conn)
        .unwrap();
    diesel::sql_query("INSERT INTO dv_unknown_arrays (moods) VALUES (ARRAY['happy'::dv_mood])")
        .execute(conn)
        .unwrap();

    let table = diesel_dynamic_schema::table("dv_unknown_arrays");
    let moods = table.column::<Array<DvMood>, _>("moods");
    let output = DynamicOutputClause::new().field(moods);
    let rows = table.select(output).load_dynamic_values(conn).unwrap();
    let array = pg_array_value(&rows[0][0]);

    match &array.values()[0] {
        DynamicValue::Backend(
            diesel_dynamic_schema::dynamic_value::pg::PgBackendValue::Opaque { bytes, .. },
        ) => assert_eq!(bytes, b"happy"),
        other => panic!("expected opaque array element, got {:?}", other),
    }
}

#[cfg(all(any(feature = "sqlite", feature = "sqlite-no-std"), feature = "chrono"))]
#[test]
fn sqlite_chrono_extension_uses_declared_metadata() {
    let conn = &mut diesel::SqliteConnection::establish(":memory:").unwrap();
    // SQLite table DDL has no typed query DSL.
    diesel::sql_query(
        "CREATE TABLE dv_sqlite_chrono_values (id INTEGER PRIMARY KEY, d DATE NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 2).unwrap();
    diesel::insert_into(dv_sqlite_chrono_values::table)
        .values(dv_sqlite_chrono_values::d.eq(date))
        .execute(conn)
        .unwrap();

    let table = diesel_dynamic_schema::table("dv_sqlite_chrono_values");
    let d = table.column::<Date, _>("d");
    let output = DynamicOutputClause::new().field(d);
    let rows = table
        .select(output)
        .load_dynamic_values_with(conn, &ChronoExtension)
        .unwrap();

    assert_eq!(rows[0][0], DynamicValue::Custom(ChronoValue::Date(date)));
}

#[cfg(all(any(feature = "sqlite", feature = "sqlite-no-std"), feature = "time"))]
#[test]
fn sqlite_time_extension_uses_declared_metadata() {
    let conn = &mut diesel::SqliteConnection::establish(":memory:").unwrap();
    // SQLite table DDL has no typed query DSL.
    diesel::sql_query(
        "CREATE TABLE dv_sqlite_time_values (id INTEGER PRIMARY KEY, d DATE NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    let date = time::Date::from_calendar_date(2000, time::Month::January, 2).unwrap();
    diesel::insert_into(dv_sqlite_time_values::table)
        .values(dv_sqlite_time_values::d.eq(date))
        .execute(conn)
        .unwrap();

    let table = diesel_dynamic_schema::table("dv_sqlite_time_values");
    let d = table.column::<Date, _>("d");
    let output = DynamicOutputClause::new().field(d);
    let rows = table
        .select(output)
        .load_dynamic_values_with(conn, &TimeExtension)
        .unwrap();

    assert_eq!(rows[0][0], DynamicValue::Custom(TimeValue::Date(date)));
}

#[cfg(all(
    any(feature = "sqlite", feature = "sqlite-no-std"),
    feature = "numeric"
))]
#[test]
fn sqlite_bigdecimal_extension_uses_declared_metadata() {
    let conn = &mut diesel::SqliteConnection::establish(":memory:").unwrap();
    // SQLite table DDL has no typed query DSL.
    diesel::sql_query(
        "CREATE TABLE dv_sqlite_numeric_values (id INTEGER PRIMARY KEY, n NUMERIC NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    let number = "12.5".parse::<bigdecimal::BigDecimal>().unwrap();
    diesel::insert_into(dv_sqlite_numeric_values::table)
        .values(dv_sqlite_numeric_values::n.eq(number.clone()))
        .execute(conn)
        .unwrap();

    let table = diesel_dynamic_schema::table("dv_sqlite_numeric_values");
    let n = table.column::<Numeric, _>("n");
    let output = DynamicOutputClause::new().field(n);
    let rows = table
        .select(output)
        .load_dynamic_values_with(conn, &BigDecimalExtension)
        .unwrap();

    assert_eq!(rows[0][0], DynamicValue::Custom(number));
}

#[cfg(all(
    any(feature = "sqlite", feature = "sqlite-no-std"),
    feature = "serde_json"
))]
#[test]
fn sqlite_json_extension_uses_declared_metadata() {
    let conn = &mut diesel::SqliteConnection::establish(":memory:").unwrap();
    // SQLite table DDL has no typed query DSL.
    diesel::sql_query(
        "CREATE TABLE dv_sqlite_json_values (id INTEGER PRIMARY KEY, j JSON NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    let json = serde_json::json!({"a": 1});
    diesel::insert_into(dv_sqlite_json_values::table)
        .values(dv_sqlite_json_values::j.eq(json.clone()))
        .execute(conn)
        .unwrap();

    let table = diesel_dynamic_schema::table("dv_sqlite_json_values");
    let j = table.column::<Json, _>("j");
    let output = DynamicOutputClause::new().field(j);
    let rows = table
        .select(output)
        .load_dynamic_values_with(conn, &SerdeJsonExtension)
        .unwrap();

    assert_eq!(rows[0][0], DynamicValue::Custom(json));
}
