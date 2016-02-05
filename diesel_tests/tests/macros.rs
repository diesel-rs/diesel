// FIXME: We need to support SQL functions on SQLite. The test itself will
// probably need to change to deal with how SQLite handles functions. I do not
// think we need to generically support creation of these functions, as it's
// different enough in SQLite to avoid.
#![cfg(feature = "postgres")]
use schema::*;
use diesel::*;
use diesel::types::VarChar;

sql_function!(my_lower, my_lower_t, (x: VarChar) -> VarChar);

#[test]
fn test_sql_function() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    connection.execute("CREATE FUNCTION my_lower(varchar) RETURNS varchar
        AS $$ SELECT LOWER($1) $$
        LANGUAGE SQL").unwrap();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(vec![sean], users.filter(my_lower(name).eq("sean"))
        .load(&connection).unwrap());
    assert_eq!(vec![tess], users.filter(my_lower(name).eq("tess"))
        .load(&connection).unwrap());
}
