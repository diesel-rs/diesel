use crate::schema::*;
use diesel::query_dsl::positional_order_dsl::PositionalOrderDsl;
use diesel::*;

#[test]
fn union() {
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
    let data: Vec<_> = users
        .filter(id.le(tess.id))
        .union(users.filter(id.ge(tess.id)))
        .positional_order_by(2) // name is the second column
        .load(conn)
        .unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn union_all() {
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
        User::new(tess.id, "Tess"),
    ];
    let data: Vec<_> = users
        .filter(id.le(tess.id))
        .union_all(users.filter(id.ge(tess.id)))
        .positional_order_by(2) // name is the second column
        .load(conn)
        .unwrap();
    assert_eq!(expected_data, data);
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn intersect() {
    use crate::schema::users::dsl::*;

    let conn = &mut connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    insert_into(users).values(&data).execute(conn).unwrap();
    let data = users.load::<User>(conn).unwrap();
    let _sean = &data[0];
    let tess = &data[1];
    let _jim = &data[2];

    let expected_data = vec![User::new(tess.id, "Tess")];
    let data: Vec<_> = users
        .filter(id.le(tess.id))
        .intersect(users.filter(id.ge(tess.id)))
        .positional_order_by(2) // name is the second column
        .load(conn)
        .unwrap();
    assert_eq!(expected_data, data);
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn except() {
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
    let _jim = &data[2];

    let expected_data = vec![User::new(sean.id, "Sean")];
    let data: Vec<_> = users
        .filter(id.le(tess.id))
        .except(users.filter(id.ge(tess.id)))
        .positional_order_by(2) // name is the second column
        .load(conn)
        .unwrap();
    assert_eq!(expected_data, data);
}
