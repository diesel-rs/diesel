use diesel::*;
use schema::connection;

mod using_infer_schema {
    use super::*;
    #[cfg(feature = "backend_specific_database_url")]
    infer_schema!("dotenv:PG_DATABASE_URL", "custom_schema");
    #[cfg(not(feature = "backend_specific_database_url"))]
    infer_schema!("dotenv:DATABASE_URL", "custom_schema");
    use self::custom_schema::users;

    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        id: i32,
    }

    #[test]
    fn custom_schemas_are_loaded_by_infer_schema() {
        let conn = connection();
        insert_into(users::table)
            .values(&NewUser { id: 1 })
            .execute(&conn)
            .unwrap();
        let users = users::table.select(users::id).load(&conn);

        assert_eq!(Ok(vec![1]), users);
    }
}

mod using_infer_table_from_schema {
    use super::*;
    mod infer_users {
        #[cfg(feature = "backend_specific_database_url")]
        infer_table_from_schema!("dotenv:PG_DATABASE_URL", "custom_schema.users");
        #[cfg(not(feature = "backend_specific_database_url"))]
        infer_table_from_schema!("dotenv:DATABASE_URL", "custom_schema.users");
    }
    use self::infer_users::users;

    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        id: i32,
    }

    #[test]
    fn custom_schemas_are_loaded_by_infer_table_from_schema() {
        let conn = connection();
        insert_into(users::table)
            .values(&NewUser { id: 1 })
            .execute(&conn)
            .unwrap();
        let users = users::table.select(users::id).load(&conn);

        assert_eq!(Ok(vec![1]), users);
    }
}

mod using_infer_table_from_schema_with_default_schema {
    use super::*;
    mod infer_users {
        #[cfg(feature = "backend_specific_database_url")]
        infer_table_from_schema!("dotenv:PG_DATABASE_URL", "users");
        #[cfg(not(feature = "backend_specific_database_url"))]
        infer_table_from_schema!("dotenv:DATABASE_URL", "users");
    }
    use self::infer_users::users;

    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser<'a> {
        id: i32,
        name: &'a str,
    }

    #[test]
    fn custom_schemas_are_loaded_by_infer_table_from_schema() {
        let conn = connection();
        insert_into(users::table)
            .values(&NewUser {
                id: 1,
                name: "Sean",
            })
            .execute(&conn)
            .unwrap();
        let users = users::table.select(users::id).load(&conn);

        assert_eq!(Ok(vec![1]), users);
    }
}
