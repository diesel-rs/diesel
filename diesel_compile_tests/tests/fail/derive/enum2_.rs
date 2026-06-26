use diesel::prelude::*;
use diesel::query_builder::QueryId;
use diesel::sql_types::SqlType;
use diesel::types::Enum;

// this should fail as we don't have any discriminant
#[derive(Debug, Enum)]
#[diesel(sql_type = diesel::sql_types::Integer)]
enum Test2 {
    A,
}

// this should fail as we don't have any discriminant
#[derive(Debug, Enum)]
#[diesel(sql_type = diesel::sql_types::Blob)]
enum Test3 {
    A,
}

#[derive(SqlType, QueryId, Clone)]
#[diesel(enum_type)]
#[diesel(postgres_type(name = "Foo"))]
#[diesel(sqlite_type(name = "Integer"))]
struct SqlEnum;

#[derive(Debug, Enum)]
#[diesel(sql_type = SqlEnum)]
enum Test4 {
    A,
}

fn main() {
    let conn = &mut SqliteConnection::establish("_").unwrap();
    let pg_conn = &mut PgConnection::establish("_").unwrap();

    let _r = diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>())
        .get_result::<Test2>(conn)
        //~^ ERROR: the trait bound `diesel::sql_types::Integer: EnumSqlType<false, _>` is not satisfied
        .unwrap();

    let _r = diesel::select(b"abc".into_sql::<diesel::sql_types::Blob>())
        .get_result::<Test2>(conn)
        //~^ ERROR: cannot deserialize a value of the database type `diesel::sql_types::Binary` as `Test2`
        .unwrap();

    // it works with a pg connection
    let _r = diesel::select(diesel::dsl::sql::<SqlEnum>("_")).get_result::<Test4>(pg_conn);
    // it fails with a sqlite connection
    let r = diesel::select(diesel::dsl::sql::<SqlEnum>("_")).get_result::<Test4>(conn);
    //~^ ERROR: `types::enum_::EnumTypeMapping` is no valid strategy to map an enum for backend `Sqlite`
}
