use crate::schema::*;
use diesel::*;

#[test]
fn delete_records() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let deleted_rows = delete(users.filter(name.eq("Sean"))).execute(connection);

    assert_eq!(Ok(1), deleted_rows);

    let num_users = users.count().first(connection);

    assert_eq!(Ok(1), num_users);
}

#[test]
fn delete_single_record() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let data = users.load::<User>(connection).unwrap();
    let sean = data[0].clone();
    let tess = data[1].clone();

    delete(&sean).execute(connection).unwrap();

    assert_eq!(Ok(vec![tess]), users.load(connection));
}

#[test]
#[cfg(not(any(
    all(feature = "sqlite", not(feature = "returning_clauses_for_sqlite_3_35")),
    feature = "mysql"
)))]
fn return_deleted_records() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let deleted_name = delete(users.filter(name.eq("Sean")))
        .returning(name)
        .get_result(connection);
    assert_eq!(Ok("Sean".to_string()), deleted_name);

    let num_users = users.count().first(connection);
    assert_eq!(Ok(1), num_users);
}

#[test]
fn delete_or_filter() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let deleted_rows =
        delete(users.filter(name.eq("Sean")).or_filter(name.eq("Tess"))).execute(connection);

    assert_eq!(Ok(2), deleted_rows);

    let num_users = users.count().first(connection);

    assert_eq!(Ok(0), num_users);
}
