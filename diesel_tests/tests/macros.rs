// FIXME: We need to support SQL functions on SQLite. The test itself will
// probably need to change to deal with how SQLite handles functions. I do not
// think we need to generically support creation of these functions, as it's
// different enough in SQLite to avoid.
#![cfg(feature = "postgres")]
use schema::*;
use diesel::*;
use diesel::sql_types::{BigInt, VarChar};

sql_function!(my_lower, my_lower_t, (x: VarChar) -> VarChar);

#[test]
fn test_sql_function() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    connection
        .execute(
            "CREATE FUNCTION my_lower(varchar) RETURNS varchar
        AS $$ SELECT LOWER($1) $$
        LANGUAGE SQL",
        )
        .unwrap();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![sean],
        users
            .filter(my_lower(name).eq("sean"))
            .load(&connection)
            .unwrap()
    );
    assert_eq!(
        vec![tess],
        users
            .filter(my_lower(name).eq("tess"))
            .load(&connection)
            .unwrap()
    );
}

sql_function!(setval, setval_t, (x: VarChar, y: BigInt));
sql_function!(currval, currval_t, (x: VarChar) -> BigInt);

#[test]
fn sql_function_without_return_type() {
    let connection = connection();
    select(setval("users_id_seq", 54))
        .execute(&connection)
        .unwrap();

    let seq_val = select(currval("users_id_seq")).get_result::<i64>(&connection);
    assert_eq!(Ok(54), seq_val);
}
