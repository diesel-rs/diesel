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
        .load(&connection).unwrap().collect::<Vec<_>>());
    assert_eq!(vec![tess], users.filter(my_lower(name).eq("tess"))
        .load(&connection).unwrap().collect::<Vec<_>>());
}
