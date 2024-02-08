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
    let data = users.order(id).load::<User>(conn).unwrap();
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
    let data = users.order(id).load::<User>(conn).unwrap();
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
    let data = users.order(name).load::<User>(conn).unwrap();
    let _sean = &data[1];
    let tess = &data[2];
    let _jim = &data[0];

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

#[test]
fn union_with_order() {
    let conn = &mut connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
        NewUser::new("Jim", None),
    ];
    insert_into(users::table)
        .values(&data)
        .execute(conn)
        .unwrap();

    let users = users::table
        .select(users::name)
        .order_by(users::id.asc())
        .limit(1)
        .union(
            users::table
                .order_by(users::id.desc())
                .select(users::name)
                .limit(1),
        )
        .positional_order_by(1)
        .load::<String>(conn)
        .unwrap();

    assert_eq!(vec![String::from("Jim"), "Sean".into()], users);
}

#[test]
fn as_subquery_for_eq_in() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    insert_into(posts::table)
        .values(&[
            (posts::user_id.eq(1), posts::title.eq("First post")),
            (posts::user_id.eq(2), posts::title.eq("Second post")),
        ])
        .execute(conn)
        .unwrap();

    let subquery = users::table
        .select(users::id)
        .filter(users::name.eq("Sean"))
        .union(
            users::table
                .select(users::id)
                .filter(users::name.ne("Sean")),
        );

    let out = posts::table
        .filter(posts::user_id.eq_any(subquery))
        .select(posts::title)
        .order_by(posts::title)
        .load::<String>(conn)
        .unwrap();

    assert_eq!(out, vec!["First post", "Second post"]);
}
