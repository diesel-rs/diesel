use crate::schema::*;
use diesel::*;

#[test]
fn order_by_column() {
    use crate::schema::users::dsl::*;

    let conn = &mut connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    insert_into(users).values(&data).execute(conn).unwrap();
    let data = users.load::<User>(conn).unwrap();
    let sean = &data[0];
    let tess = &data[1];
    let jim = &data[2];

    let expected_data = vec![
        User::new(jim.id, "Jim"),
        User::new(sean.id, "Sean"),
        User::new(tess.id, "Tess"),
    ];
    let data: Vec<_> = users.order(name).load(conn).unwrap();
    assert_eq!(expected_data, data);

    insert_into(users)
        .values(&NewUser::new("Aaron", None))
        .execute(conn)
        .unwrap();
    let aaron = users.order(id.desc()).first::<User>(conn).unwrap();
    let expected_data = vec![
        User::new(aaron.id, "Aaron"),
        User::new(jim.id, "Jim"),
        User::new(sean.id, "Sean"),
        User::new(tess.id, "Tess"),
    ];
    let data: Vec<_> = users.order(name.asc()).load(conn).unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn order_by_descending_column() {
    use crate::schema::users::dsl::*;

    let conn = &mut connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    insert_into(users).values(&data).execute(conn).unwrap();
    let data = users.load::<User>(conn).unwrap();
    let sean = &data[0];
    let tess = &data[1];
    let jim = &data[2];

    let expected_data = vec![
        User::new(tess.id, "Tess"),
        User::new(sean.id, "Sean"),
        User::new(jim.id, "Jim"),
    ];
    let data: Vec<_> = users.order(name.desc()).load(conn).unwrap();
    assert_eq!(expected_data, data);

    insert_into(users)
        .values(&NewUser::new("Aaron", None))
        .execute(conn)
        .unwrap();
    let aaron = users.order(id.desc()).first::<User>(conn).unwrap();
    let expected_data = vec![
        User::new(tess.id, "Tess"),
        User::new(sean.id, "Sean"),
        User::new(jim.id, "Jim"),
        User::new(aaron.id, "Aaron"),
    ];
    let data: Vec<_> = users.order(name.desc()).load(conn).unwrap();
    assert_eq!(expected_data, data);
}
