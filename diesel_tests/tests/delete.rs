use crate::schema::*;
use diesel::*;

#[test]
fn delete_records() {
    use crate::schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    let deleted_rows = delete(users.filter(name.eq("Sean"))).execute(&connection);

    assert_eq!(Ok(1), deleted_rows);

    let num_users = users.count().first(&connection);

    assert_eq!(Ok(1), num_users);
}

#[test]
fn delete_single_record() {
    use crate::schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();
    let data = users.load::<User>(&connection).unwrap();
    let sean = data[0].clone();
    let tess = data[1].clone();

    delete(&sean).execute(&connection).unwrap();

    assert_eq!(Ok(vec![tess]), users.load(&connection));
}

#[test]
#[cfg(not(any(feature = "sqlite", feature = "mysql")))]
fn return_deleted_records() {
    use crate::schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    let deleted_name = delete(users.filter(name.eq("Sean")))
        .returning(name)
        .get_result(&connection);
    assert_eq!(Ok("Sean".to_string()), deleted_name);

    let num_users = users.count().first(&connection);
    assert_eq!(Ok(1), num_users);
}

#[test]
#[cfg(feature = "postgres")]
fn delete_with_returning_into_selectable() {
    #[derive(Insertable, Queryable, Selectable, Debug, PartialEq)]
    #[table_name = "users"]
    pub struct User {
        pub name: String,
        pub hair_color: Option<String>,
    }

    let connection = connection_with_sean_and_tess_in_users_table();
    let deleted_user = User {
        name: "Sean".to_string(),
        hair_color: None,
    };

    let result = delete(users::table.filter(users::name.eq("Sean")))
        .load_into_single::<User>(&connection)
        .unwrap();

    assert_eq!(deleted_user, result);
}
