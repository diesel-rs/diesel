use schema::*;
use diesel::*;

#[test]
fn order_by_column() {
    use schema::users::dsl::*;

    let conn = connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    let data: Vec<User> =
        insert(&data).into(users).get_results(&conn).unwrap().collect();
    let sean = &data[0];
    let tess = &data[1];
    let jim = &data[2];

    let expected_data = vec![
        User::new(jim.id, "Jim"),
        User::new(sean.id, "Sean"),
        User::new(tess.id, "Tess"),
    ];
    let data: Vec<_> = users.order(name).load(&conn).unwrap().collect();
    assert_eq!(expected_data, data);

    let aaron = insert(&NewUser::new("Aaron", None)).into(users)
        .get_result::<User>(&conn).unwrap();
    let expected_data = vec![
        User::new(aaron.id, "Aaron"),
        User::new(jim.id, "Jim"),
        User::new(sean.id, "Sean"),
        User::new(tess.id, "Tess"),
    ];
    let data: Vec<_> = users.order(name.asc()).load(&conn).unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn order_by_descending_column() {
    use schema::users::dsl::*;

    let conn = connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    let data: Vec<User> = insert(&data).into(users)
        .get_results(&conn).unwrap().collect();
    let sean = &data[0];
    let tess = &data[1];
    let jim = &data[2];

    let expected_data = vec![
        User::new(tess.id, "Tess"),
        User::new(sean.id, "Sean"),
        User::new(jim.id, "Jim"),
    ];
    let data: Vec<_> = users.order(name.desc()).load(&conn).unwrap().collect();
    assert_eq!(expected_data, data);

    let aaron = insert(&NewUser::new("Aaron", None)).into(users)
        .get_result::<User>(&conn).unwrap();
    let expected_data = vec![
        User::new(tess.id, "Tess"),
        User::new(sean.id, "Sean"),
        User::new(jim.id, "Jim"),
        User::new(aaron.id, "Aaron"),
    ];
    let data: Vec<_> = users.order(name.desc()).load(&conn).unwrap().collect();
    assert_eq!(expected_data, data);
}
