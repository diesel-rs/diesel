use crate::schema::*;
use diesel::*;

#[cfg(feature = "postgres")]
table! {
    bpchar_pk_table (id) {
        id -> diesel::pg::sql_types::Bpchar,
        value -> Integer,
    }
}

#[cfg(feature = "postgres")]
table! {
    bpchar_col_table (id) {
        id -> Integer,
        code -> diesel::pg::sql_types::Bpchar,
    }
}

#[cfg(feature = "postgres")]
#[derive(diesel::QueryableByName)]
struct ExplainRow {
    #[diesel(sql_type = diesel::sql_types::Text, column_name = "QUERY PLAN")]
    query_plan: String,
}

#[cfg(feature = "postgres")]
fn explain(query: &str, conn: &mut diesel::PgConnection) -> String {
    let rows: Vec<ExplainRow> = diesel::sql_query(format!("EXPLAIN {query}"))
        .load(conn)
        .unwrap();
    rows.iter()
        .map(|r| r.query_plan.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn find_by_bpchar_pk() {
    use self::bpchar_pk_table::dsl::*;
    let conn = &mut connection();
    diesel::sql_query(
        "CREATE TABLE bpchar_pk_table (id CHAR(11) PRIMARY KEY, value INTEGER NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query("INSERT INTO bpchar_pk_table (id, value) VALUES ('00005000000', 42)")
        .execute(conn)
        .unwrap();

    let result = bpchar_pk_table
        .find("00005000000")
        .first::<(String, i32)>(conn);
    assert_eq!(Ok(("00005000000".to_string(), 42)), result);
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_by_bpchar_column() {
    use self::bpchar_pk_table::dsl::*;
    let conn = &mut connection();
    diesel::sql_query(
        "CREATE TABLE bpchar_pk_table (id CHAR(11) PRIMARY KEY, value INTEGER NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO bpchar_pk_table (id, value) VALUES ('00005000000', 42), ('00001000000', 7)",
    )
    .execute(conn)
    .unwrap();

    let result = bpchar_pk_table
        .filter(id.eq("00005000000"))
        .first::<(String, i32)>(conn);
    assert_eq!(Ok(("00005000000".to_string(), 42)), result);
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn bpchar_bind_uses_index() {
    let conn = &mut connection();
    diesel::sql_query(
        "CREATE TABLE bpchar_pk_table (id CHAR(11) PRIMARY KEY, value INTEGER NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query("INSERT INTO bpchar_pk_table (id, value) VALUES ('00005000000', 42)")
        .execute(conn)
        .unwrap();

    let plan = explain(
        "SELECT * FROM bpchar_pk_table WHERE id = '00005000000'",
        conn,
    );
    assert!(
        plan.contains("Index Scan using bpchar_pk_table_pkey on bpchar_pk_table"),
        "expected primary key index scan, got:\n{plan}",
    );
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_by_non_pk_bpchar_column() {
    use self::bpchar_col_table::dsl::*;
    let conn = &mut connection();
    diesel::sql_query(
        "CREATE TABLE bpchar_col_table (id INTEGER PRIMARY KEY, code CHAR(11) NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query("CREATE INDEX ON bpchar_col_table (code)")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO bpchar_col_table (id, code) VALUES (1, '00005000000'), (2, '00001000000')",
    )
    .execute(conn)
    .unwrap();

    let result = bpchar_col_table
        .filter(code.eq("00005000000"))
        .first::<(i32, String)>(conn);
    assert_eq!(Ok((1, "00005000000".to_string())), result);
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn non_pk_bpchar_column_uses_index() {
    let conn = &mut connection();
    diesel::sql_query(
        "CREATE TABLE bpchar_col_table (id INTEGER PRIMARY KEY, code CHAR(11) NOT NULL)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query("CREATE INDEX ON bpchar_col_table (code)")
        .execute(conn)
        .unwrap();
    diesel::sql_query("INSERT INTO bpchar_col_table (id, code) VALUES (1, '00005000000')")
        .execute(conn)
        .unwrap();

    let plan = explain(
        "SELECT * FROM bpchar_col_table WHERE code = '00005000000'",
        conn,
    );
    assert!(
        plan.contains("Bitmap Index Scan on bpchar_col_table_code_idx"),
        "expected bitmap index scan on bpchar_col_table_code_idx, got:\n{plan}",
    );
}

#[diesel_test_helper::test]
fn find() {
    use crate::schema::users::table as users;

    let connection = &mut connection();

    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .execute(connection)
        .unwrap();

    assert_eq!(Ok(User::new(1, "Sean")), users.find(1).first(connection));
    assert_eq!(Ok(User::new(2, "Tess")), users.find(2).first(connection));
    assert_eq!(Ok(None::<User>), users.find(3).first(connection).optional());
}

table! {
    users_with_name_pk (name) {
        name -> VarChar,
    }
}

#[diesel_test_helper::test]
fn find_with_non_serial_pk() {
    use self::users_with_name_pk::table as users;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users_with_name_pk (name) VALUES ('Sean'), ('Tess')")
        .execute(connection)
        .unwrap();

    assert_eq!(
        Ok(("Sean".to_string(),),),
        users.find("Sean").first(connection)
    );
    assert_eq!(
        Ok(("Tess".to_string(),),),
        users.find("Tess".to_string()).first(connection)
    );
    assert_eq!(
        Ok(None::<(String,)>),
        users.find("Wibble").first(connection).optional()
    );
}

#[diesel_test_helper::test]
fn find_with_composite_pk() {
    use crate::schema::followings::dsl::*;

    let first_following = Following {
        user_id: 1,
        post_id: 1,
        email_notifications: true,
    };
    let second_following = Following {
        user_id: 1,
        post_id: 2,
        email_notifications: false,
    };
    let third_following = Following {
        user_id: 2,
        post_id: 1,
        email_notifications: false,
    };

    let connection = &mut connection();
    disable_foreign_keys(connection);
    insert_into(followings)
        .values(&vec![first_following, second_following, third_following])
        .execute(connection)
        .unwrap();

    assert_eq!(
        Ok(first_following),
        followings.find((1, 1)).first(connection)
    );
    assert_eq!(
        Ok(second_following),
        followings.find((1, 2)).first(connection)
    );
    assert_eq!(
        Ok(third_following),
        followings.find((2, 1)).first(connection)
    );
    assert_eq!(
        Ok(None::<Following>),
        followings.find((2, 2)).first(connection).optional()
    );
}

#[diesel_test_helper::test]
fn select_then_find() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = users.select(name).find(1).first(connection);
    let tess = users.select(name).find(2).first(connection);

    assert_eq!(Ok(String::from("Sean")), sean);
    assert_eq!(Ok(String::from("Tess")), tess);
}

#[diesel_test_helper::test]
fn select_by_then_find() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = users
        .select(UserName::as_select())
        .find(1)
        .first(connection);
    let tess = users
        .select(UserName::as_select())
        .find(2)
        .first(connection);

    assert_eq!(Ok(UserName::new("Sean")), sean);
    assert_eq!(Ok(UserName::new("Tess")), tess);
}
