// FIXME: We need to support SQL functions on SQLite. The test itself will
// probably need to change to deal with how SQLite handles functions. I do not
// think we need to generically support creation of these functions, as it's
// different enough in SQLite to avoid.
#![cfg(feature = "postgres")]
use schema::*;
use diesel::*;
use diesel::sql_types::{BigInt, Integer, VarChar};

sql_function!(my_substring, my_substring_t, (x: VarChar, y: Integer) -> VarChar);
variant_sql_function!(my_substring_with_length, my_substring_t2, my_substring, (x: VarChar, y: Integer, z: Integer) -> VarChar);

#[test]
fn test_sql_function() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    connection
        .execute(
            "CREATE FUNCTION my_substring(varchar, integer) RETURNS varchar
        AS $$ SELECT SUBSTRING($1 from $2) $$
        LANGUAGE SQL",
        )
        .unwrap();
    connection
        .execute(
            "CREATE FUNCTION my_substring(varchar, integer, integer) RETURNS varchar
            AS $$ SELECT SUBSTRING($1 from $2 for $3)$$
            LANGUAGE SQL",
        )
        .unwrap();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![sean],
        users
            .filter(my_substring(name, 3).eq("an"))
            .load(&connection)
            .unwrap()
    );
    assert_eq!(
        vec![tess],
        users
            .filter(my_substring(name, 3).eq("ss"))
            .load(&connection)
            .unwrap()
    );

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![sean],
        users
            .filter(my_substring_with_length(name, 3, 1).eq("a"))
            .load(&connection)
            .unwrap()
    );
    assert_eq!(
        vec![tess],
        users
            .filter(my_substring_with_length(name, 2, 2).eq("es"))
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
