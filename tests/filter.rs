use schema::*;
use yaqb::*;

#[test]
fn filter_by_int_equality() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    let data = [NewUser::new("Sean", None), NewUser::new("Tess", None)];
    connection.insert_without_return(&users, &data).unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    assert_eq!(Some(sean), connection.query_one(&users.filter(id.eq(1))).unwrap());
    assert_eq!(Some(tess), connection.query_one(&users.filter(id.eq(2))).unwrap());
    assert_eq!(None::<User>, connection.query_one(&users.filter(id.eq(3))).unwrap());
}

#[test]
fn filter_by_string_equality() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    let data = [NewUser::new("Sean", None), NewUser::new("Tess", None)];
    connection.insert_without_return(&users, &data).unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    assert_eq!(Some(sean), connection.query_one(&users.filter(name.eq("Sean"))).unwrap());
    assert_eq!(Some(tess), connection.query_one(&users.filter(name.eq("Tess"))).unwrap());
    assert_eq!(None::<User>, connection.query_one(&users.filter(name.eq("Jim"))).unwrap());
}
