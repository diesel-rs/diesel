use schema::*;
use diesel::*;

#[test]
fn order_by_column() {
    use schema::users::dsl::*;

    let conn = connection();
    let data = vec![
        NewUser::new("Sean", None, UserType::Default),
        NewUser::new("Tess", None, UserType::Default),
        NewUser::new("Jim", None, UserType::Default),
    ];
    insert(&data).into(users).execute(&conn).unwrap();
    let data = users.load::<User>(&conn).unwrap();
    let sean = &data[0];
    let tess = &data[1];
    let jim = &data[2];

    let expected_data = vec![
        User::new(jim.id, "Jim", UserType::Default),
        User::new(sean.id, "Sean", UserType::Default),
        User::new(tess.id, "Tess", UserType::Default),
    ];
    let data: Vec<_> = users.order(name).load(&conn).unwrap();
    assert_eq!(expected_data, data);

    insert(&NewUser::new("Aaron", None, UserType::Default)).into(users)
        .execute(&conn).unwrap();
    let aaron = users.order(id.desc()).first::<User>(&conn).unwrap();
    let expected_data = vec![
        User::new(aaron.id, "Aaron", UserType::Default),
        User::new(jim.id, "Jim", UserType::Default),
        User::new(sean.id, "Sean", UserType::Default),
        User::new(tess.id, "Tess", UserType::Default),
    ];
    let data: Vec<_> = users.order(name.asc()).load(&conn).unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn order_by_descending_column() {
    use schema::users::dsl::*;

    let conn = connection();
    let data = vec![
        NewUser::new("Sean", None, UserType::Default),
        NewUser::new("Tess", None, UserType::Default),
        NewUser::new("Jim", None, UserType::Default),
    ];
    insert(&data).into(users).execute(&conn).unwrap();
    let data = users.load::<User>(&conn).unwrap();
    let sean = &data[0];
    let tess = &data[1];
    let jim = &data[2];

    let expected_data = vec![
        User::new(tess.id, "Tess", UserType::Default),
        User::new(sean.id, "Sean", UserType::Default),
        User::new(jim.id, "Jim", UserType::Default),
    ];
    let data: Vec<_> = users.order(name.desc()).load(&conn).unwrap();
    assert_eq!(expected_data, data);

    insert(&NewUser::new("Aaron", None, UserType::Default)).into(users)
        .execute(&conn).unwrap();
    let aaron = users.order(id.desc()).first::<User>(&conn).unwrap();
    let expected_data = vec![
        User::new(tess.id, "Tess", UserType::Default),
        User::new(sean.id, "Sean", UserType::Default),
        User::new(jim.id, "Jim", UserType::Default),
        User::new(aaron.id, "Aaron", UserType::Default),
    ];
    let data: Vec<_> = users.order(name.desc()).load(&conn).unwrap();
    assert_eq!(expected_data, data);
}
