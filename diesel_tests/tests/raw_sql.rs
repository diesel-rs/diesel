use crate::schema::*;
use diesel::*;

#[test]
fn execute_query_by_raw_sql() {
    let conn = &mut connection();

    let inserted_rows = sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean')").execute(conn);
    let users = users::table.load(conn);
    let expected_users = vec![User::new(1, "Sean")];

    assert_eq!(Ok(1), inserted_rows);
    assert_eq!(Ok(expected_users), users);
}

#[test]
fn query_by_raw_sql() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", conn);
    let tess = find_user_by_name("Tess", conn);

    let users = sql_query("SELECT * FROM users ORDER BY id").load(conn);
    let expected = vec![sean, tess];
    assert_eq!(Ok(expected), users);
}

#[test]
fn sql_query_deserializes_by_name_not_index() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", conn);
    let tess = find_user_by_name("Tess", conn);

    let users = sql_query("SELECT name, hair_color, id FROM users ORDER BY id").load(conn);
    let expected = vec![sean, tess];
    assert_eq!(Ok(expected), users);
}

#[test]
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

#[test]
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
