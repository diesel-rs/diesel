use schema::*;
use yaqb::*;

#[test]
fn order_by_column() {
    use schema::users::dsl::*;

    let conn = connection();
    setup_users_table(&conn);
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    conn.insert_without_return(&users, &data).unwrap();

    let expected_data = vec![
        User::new(3, "Jim"),
        User::new(1, "Sean"),
        User::new(2, "Tess"),
    ];
    let data: Vec<_> = conn.query_all(users.order(name)).unwrap().collect();
    assert_eq!(expected_data, data);

    conn.insert_without_return(&users, &[NewUser::new("Aaron", None)]).unwrap();
    let expected_data = vec![
        User::new(4, "Aaron"),
        User::new(3, "Jim"),
        User::new(1, "Sean"),
        User::new(2, "Tess"),
    ];
    let data: Vec<_> = conn.query_all(users.order(name)).unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn order_by_descending_column() {
    use schema::users::dsl::*;

    let conn = connection();
    setup_users_table(&conn);
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    conn.insert_without_return(&users, &data).unwrap();

    let expected_data = vec![
        User::new(2, "Tess"),
        User::new(1, "Sean"),
        User::new(3, "Jim"),
    ];
    let data: Vec<_> = conn.query_all(users.order(name.desc())).unwrap().collect();
    assert_eq!(expected_data, data);

    conn.insert_without_return(&users, &[NewUser::new("Aaron", None)]).unwrap();
    let expected_data = vec![
        User::new(2, "Tess"),
        User::new(1, "Sean"),
        User::new(3, "Jim"),
        User::new(4, "Aaron"),
    ];
    let data: Vec<_> = conn.query_all(users.order(name.desc())).unwrap().collect();
    assert_eq!(expected_data, data);
}
