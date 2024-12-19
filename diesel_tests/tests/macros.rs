// FIXME: We need to support SQL functions on SQLite. The test itself will
// probably need to change to deal with how SQLite handles functions. I do not
// think we need to generically support creation of these functions, as it's
// different enough in SQLite to avoid.
#![cfg(feature = "postgres")]
use crate::schema::*;
use diesel::sql_types::{BigInt, VarChar};
use diesel::*;

#[declare_sql_function]
extern "SQL" {
    fn my_lower(x: VarChar) -> VarChar;
    fn setval(x: VarChar, y: BigInt);
    fn currval(x: VarChar) -> BigInt;
}

#[test]
fn test_sql_function() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    diesel::sql_query(
        "CREATE FUNCTION my_lower(varchar) RETURNS varchar
        AS $$ SELECT LOWER($1) $$
        LANGUAGE SQL",
    )
    .execute(connection)
    .unwrap();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![sean],
        users
            .filter(my_lower(name).eq("sean"))
            .load(connection)
            .unwrap()
    );
    assert_eq!(
        vec![tess],
        users
            .filter(my_lower(name).eq("tess"))
            .load(connection)
            .unwrap()
    );
}

#[test]
fn sql_function_without_return_type() {
    let connection = &mut connection();
    select(setval("users_id_seq", 54))
        .execute(connection)
        .unwrap();

    let seq_val = select(currval("users_id_seq")).get_result::<i64>(connection);
    assert_eq!(Ok(54), seq_val);
}
