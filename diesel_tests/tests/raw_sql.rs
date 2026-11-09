use crate::schema::*;
use diesel::sql_types::Integer;
use diesel::*;

// This test reads query results, which requires reading memory
// allocated by the native sqlite library. Miri cannot track the
// provenance of that memory when using `-Zmiri-native-lib`.
#[cfg(not(miri))]
#[diesel_test_helper::test]
fn execute_query_by_raw_sql() {
    let conn = &mut connection();

    let inserted_rows = sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean')").execute(conn);
    let users = users::table.load(conn);
    let expected_users = vec![User::new(1, "Sean")];

    assert_eq!(Ok(1), inserted_rows);
    assert_eq!(Ok(expected_users), users);
}

// This test reads query results, which requires reading memory
// allocated by the native sqlite library. Miri cannot track the
// provenance of that memory when using `-Zmiri-native-lib`.
#[cfg(not(miri))]
#[diesel_test_helper::test]
fn query_by_raw_sql() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", conn);
    let tess = find_user_by_name("Tess", conn);

    let users = sql_query("SELECT * FROM users ORDER BY id").load(conn);
    let expected = vec![sean, tess];
    assert_eq!(Ok(expected), users);
}

// This test reads query results, which requires reading memory
// allocated by the native sqlite library. Miri cannot track the
// provenance of that memory when using `-Zmiri-native-lib`.
#[cfg(not(miri))]
#[diesel_test_helper::test]
fn sql_query_deserializes_by_name_not_index() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", conn);
    let tess = find_user_by_name("Tess", conn);

    let users = sql_query("SELECT name, hair_color, id FROM users ORDER BY id").load(conn);
    let expected = vec![sean, tess];
    assert_eq!(Ok(expected), users);
}

// This test reads query results, which requires reading memory
// allocated by the native sqlite library. Miri cannot track the
// provenance of that memory when using `-Zmiri-native-lib`.
#[cfg(not(miri))]
#[diesel_test_helper::test]
fn sql_query_can_take_bind_params() {
    use diesel::sql_types::Text;

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let tess = find_user_by_name("Tess", conn);

    let query = if cfg!(feature = "postgres") {
        sql_query("SELECT * FROM users WHERE name = $1")
    } else {
        sql_query("SELECT * FROM users WHERE name = ?")
    };
    let users = query.bind::<Text, _>("Tess").load(conn);
    let expected = vec![tess];

    assert_eq!(Ok(expected), users);
}

// This test reads query results, which requires reading memory
// allocated by the native sqlite library. Miri cannot track the
// provenance of that memory when using `-Zmiri-native-lib`.
#[cfg(not(miri))]
#[diesel_test_helper::test]
fn sql_query_can_take_bind_params_boxed() {
    use diesel::sql_types::Text;

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let tess = find_user_by_name("Tess", conn);

    let mut query = sql_query("SELECT * FROM users ").into_boxed();

    let mut where_prefix = Some("WHERE ( ");
    let mut get_where_prefix = || where_prefix.take().unwrap_or("AND ( ");

    let bind_char = if cfg!(feature = "postgres") {
        "$1"
    } else {
        "?"
    };

    let users_not = vec!["Sean", "Bill", "Bob"];
    for user in users_not {
        query = query
            .sql(get_where_prefix())
            .sql("name != ")
            .sql(bind_char)
            .sql(") ")
            .bind::<Text, _>(user);
    }

    let users = query.load(conn);
    let expected = vec![tess];

    assert_eq!(Ok(expected), users);
}

#[diesel_test_helper::test]
fn fewer_binds_than_placeholders() {
    let conn = &mut connection();
    let query = if cfg!(feature = "postgres") {
        diesel::sql_query("SELECT $1, $2, $3, $4")
    } else {
        diesel::sql_query("SELECT ?, ?, ?, ?")
    };
    let r = query.bind::<Integer, _>(1).execute(conn);
    assert!(r.is_err(), "expected an error for missing binds, got {r:?}");

    assert_eq!(
        r.unwrap_err().to_string(),
        if cfg!(feature = "postgres") {
            "bind message supplies 1 parameters, but prepared statement \"\" requires 4"
        } else {
            "prepared statement expected a different number of parameters than provided: Expected 4, provided: 1"
        }
    );
}

#[diesel_test_helper::test]
fn no_binds_for_placeholder() {
    let conn = &mut connection();
    let query = if cfg!(feature = "postgres") {
        diesel::sql_query("SELECT $1")
    } else {
        diesel::sql_query("SELECT ?")
    };
    let r = query.execute(conn);
    assert!(r.is_err(), "expected an error for missing binds, got {r:?}");
    assert_eq!(
        r.unwrap_err().to_string(),
        if cfg!(feature = "postgres") {
            "bind message supplies 0 parameters, but prepared statement \"\" requires 1"
        } else {
            "prepared statement expected a different number of parameters than provided: Expected 1, provided: 0"
        }
    );
}
