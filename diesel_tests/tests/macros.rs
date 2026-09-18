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

#[diesel_test_helper::test]
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

#[diesel_test_helper::test]
fn sql_function_without_return_type() {
    let connection = &mut connection();
    select(setval("users_id_seq", 54))
        .execute(connection)
        .unwrap();

    let seq_val = select(currval("users_id_seq")).get_result::<i64>(connection);
    assert_eq!(Ok(54), seq_val);
}

#[cfg(feature = "postgres")]
mod named_params_block_pg {
    use diesel::sql_types::{BigInt, VarChar};
    use diesel::*;

    #[declare_sql_function(named_parameters = true)]
    extern "SQL" {
        fn has_named_parameters(a: BigInt, b: VarChar);
        #[named_parameters = false]
        fn has_positional_parameters(a: BigInt, b: VarChar);
    }

    #[diesel_test_helper::test]
    fn has_named_parameters_sql() {
        insta::assert_snapshot!(
            diesel::debug_query::<diesel::pg::Pg, _>(&has_named_parameters(10, "text")).to_string()
        );
    }

    #[diesel_test_helper::test]
    fn has_positional_parameters_sql() {
        insta::assert_snapshot!(
            diesel::debug_query::<diesel::pg::Pg, _>(&has_positional_parameters(10, "text"))
                .to_string()
        );
    }
}

#[cfg(feature = "postgres")]
mod positional_params_block_pg {
    use diesel::sql_types::{BigInt, VarChar};
    use diesel::*;

    #[declare_sql_function]
    extern "SQL" {
        #[named_parameters = true]
        fn has_named_parameters(a: BigInt, b: VarChar);
        fn has_positional_parameters(a: BigInt, b: VarChar);
    }

    #[diesel_test_helper::test]
    fn has_named_parameters_sql() {
        insta::assert_snapshot!(
            diesel::debug_query::<diesel::pg::Pg, _>(&has_named_parameters(10, "text")).to_string()
        );
    }

    #[diesel_test_helper::test]
    fn has_positional_parameters_sql() {
        insta::assert_snapshot!(
            diesel::debug_query::<diesel::pg::Pg, _>(&has_positional_parameters(10, "text"))
                .to_string()
        );
    }
}

#[cfg(not(feature = "postgres"))]
mod named_params_block_other {
    use diesel::sql_types::{BigInt, VarChar};
    use diesel::*;

    #[cfg(feature = "sqlite")]
    pub type TestConnection = SqliteConnection;
    #[cfg(feature = "mysql")]
    pub type TestConnection = MysqlConnection;
    #[cfg(feature = "mariadb")]
    pub type TestConnection = MariadbConnection;

    pub type TestBackend = <TestConnection as Connection>::Backend;

    #[declare_sql_function(named_parameters = true)]
    extern "SQL" {
        // Named parameters are only supported on Postgres, so this function should use
        // positional parameters
        fn has_positional_parameters(a: BigInt, b: VarChar);
    }

    #[diesel_test_helper::test]
    fn has_positional_parameters_sql() {
        insta::assert_snapshot!(
            diesel::debug_query::<TestBackend, _>(&has_positional_parameters(10, "text"))
                .to_string()
        );
    }
}

#[cfg(not(feature = "postgres"))]
mod positional_params_block_other {
    use diesel::sql_types::{BigInt, VarChar};
    use diesel::*;

    #[cfg(feature = "sqlite")]
    pub type TestConnection = SqliteConnection;
    #[cfg(feature = "mysql")]
    pub type TestConnection = MysqlConnection;
    #[cfg(feature = "mariadb")]
    pub type TestConnection = MariadbConnection;

    pub type TestBackend = <TestConnection as Connection>::Backend;

    #[declare_sql_function]
    extern "SQL" {
        // Named parameters are only supported on Postgres, so this function should use
        // positional parameters
        #[named_parameters = true]
        fn has_positional_parameters(a: BigInt, b: VarChar);
    }

    #[diesel_test_helper::test]
    fn has_positional_parameters_sql() {
        insta::assert_snapshot!(
            diesel::debug_query::<TestBackend, _>(&has_positional_parameters(10, "text"))
                .to_string()
        );
    }
}
