use crate::connection;
use assert_matches::assert_matches;
use diesel::dsl::{delete, insert_into, sql};
use diesel::pg::sql_state;
use diesel::result::{DatabaseErrorKind, Error};
use diesel::sql_types::Integer;
use diesel::{table, Insertable, IntoSql};
use diesel_async::{RunQueryDsl, SimpleAsyncConnection};

table! {
    use diesel::sql_types::*;
    parent {
        id -> Integer,
    }
}

table! {
    use diesel::sql_types::*;
    child {
        id -> Integer,
        parent_id -> Integer,
    }
}

#[derive(Insertable, Debug)]
#[diesel(table_name = parent)]
struct Parent {
    id: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = child)]
struct Child {
    id: i32,
    parent_id: i32,
}

#[tokio::test]
async fn unique_violation_error_kind() {
    let connection = &mut connection().await;

    diesel::sql_query("CREATE TABLE parent (id INTEGER PRIMARY KEY)")
        .execute(connection)
        .await
        .unwrap();

    insert_into(parent::table)
        .values(&[Parent { id: 1 }])
        .execute(connection)
        .await
        .unwrap();

    let err = insert_into(parent::table)
        .values(&[Parent { id: 1 }])
        .execute(connection)
        .await
        .unwrap_err();

    assert_matches!(err, Error::DatabaseError(kind, _) if kind == DatabaseErrorKind::UniqueViolation);
}

#[tokio::test]
async fn foreign_key_violation_error_kind() {
    let connection = &mut connection().await;

    connection
        .batch_execute(
            r#"
            CREATE TABLE parent (id INTEGER PRIMARY KEY);
            CREATE TABLE child (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER REFERENCES parent(id) ON DELETE RESTRICT
            );
        "#,
        )
        .await
        .unwrap();

    let err = insert_into(child::table)
        .values(&[Child {
            id: 0,
            parent_id: 0,
        }])
        .execute(connection)
        .await
        .unwrap_err();

    assert_matches!(err, Error::DatabaseError(kind, _) if kind == DatabaseErrorKind::ForeignKeyViolation);
}

#[tokio::test]
async fn restrict_violation_error_kind() {
    let connection = &mut connection().await;

    connection
        .batch_execute(
            r#"
            CREATE TABLE parent (id INTEGER PRIMARY KEY);
            CREATE TABLE child (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER REFERENCES parent(id) ON DELETE RESTRICT
            );
        "#,
        )
        .await
        .unwrap();

    insert_into(parent::table)
        .values(&[Parent { id: 1 }])
        .execute(connection)
        .await
        .unwrap();

    insert_into(child::table)
        .values(&[Child {
            id: 0,
            parent_id: 1,
        }])
        .execute(connection)
        .await
        .unwrap();

    let pg_version_num = diesel::select(sql::<Integer>(
        "current_setting('server_version_num')::integer",
    ))
    .get_result::<i32>(connection)
    .await
    .unwrap();

    let err = delete(parent::table).execute(connection).await.unwrap_err();

    // Postgres 18 changed the error code raised by the RESTRICT referential
    // action from `FOREIGN KEY VIOLATION` to `RESTRICT VIOLATION`, see
    // https://github.com/postgres/postgres/commit/086c84b23d
    let expected_kind = if pg_version_num >= 180_000 {
        DatabaseErrorKind::RestrictViolation
    } else {
        DatabaseErrorKind::ForeignKeyViolation
    };
    assert_matches!(err, Error::DatabaseError(kind, _) if kind == expected_kind);
}

#[tokio::test]
async fn exclusion_violation_error_kind() {
    let connection = &mut connection().await;

    diesel::sql_query(
        r#"
            CREATE TABLE parent (
                id INTEGER,
                EXCLUDE USING btree (id WITH =)
            )
        "#,
    )
    .execute(connection)
    .await
    .unwrap();

    insert_into(parent::table)
        .values(&[Parent { id: 1 }])
        .execute(connection)
        .await
        .unwrap();

    let err = insert_into(parent::table)
        .values(&[Parent { id: 1 }])
        .execute(connection)
        .await
        .unwrap_err();

    assert_matches!(err, Error::DatabaseError(kind, _) if kind == DatabaseErrorKind::ExclusionViolation);
}

#[tokio::test]
async fn not_null_violation_error_kind() {
    let connection = &mut connection().await;
    diesel::sql_query(
        r#"
            CREATE TABLE users_not_null_name (
                id INTEGER,
                name TEXT NOT NULL
            )
        "#,
    )
    .execute(connection)
    .await
    .unwrap();

    let err = diesel::sql_query("INSERT INTO users_not_null_name (id, name) VALUES (0, NULL)")
        .execute(connection)
        .await
        .unwrap_err();

    assert_matches!(err, Error::DatabaseError(kind, _) if kind == DatabaseErrorKind::NotNullViolation);
}

#[tokio::test]
async fn check_violation_error_kind() {
    let connection = &mut connection().await;
    diesel::sql_query(
        r#"
            CREATE TABLE parent (
                id INTEGER PRIMARY KEY,
                CHECK ( id = 0 )
            )
        "#,
    )
    .execute(connection)
    .await
    .unwrap();

    let err = insert_into(parent::table)
        .values(&[Parent { id: 1 }])
        .execute(connection)
        .await
        .unwrap_err();

    assert_matches!(err, Error::DatabaseError(kind, _) if kind == DatabaseErrorKind::CheckViolation);
}

#[tokio::test]
async fn sqlstate_reports_the_code_of_an_unmapped_error() {
    let connection = &mut connection().await;

    let err = diesel::select(1.into_sql::<Integer>() / 0.into_sql::<Integer>())
        .get_result::<i32>(connection)
        .await
        .unwrap_err();

    let Error::DatabaseError(DatabaseErrorKind::Unknown, info) = err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(info.sqlstate(), Some(sql_state::DIVISION_BY_ZERO));
}

#[tokio::test]
async fn sqlstate_reports_a_custom_code_raised_by_plpgsql() {
    let connection = &mut connection().await;

    // Raising an error with a custom code needs PL/pgSQL, which the DSL cannot express.
    let err = connection
        .batch_execute(
            "DO $$ BEGIN RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'custom', DETAIL = 'kept'; END $$",
        )
        .await
        .unwrap_err();

    let Error::DatabaseError(DatabaseErrorKind::Unknown, info) = err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(info.sqlstate().unwrap().as_str(), "P1001");
    assert_eq!(info.details(), Some("kept"));
}
