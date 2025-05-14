use crate::schema::*;
use diesel::query_dsl::positional_order_dsl::{OrderColumn, PositionalOrderDsl};
use diesel::*;

#[diesel_test_helper::test]
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

#[diesel_test_helper::test]
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

#[diesel_test_helper::test]
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

#[diesel_test_helper::test]
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

#[diesel_test_helper::test]
fn union_with_limit() {
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
    let _jim = &data[2];

    let data: Vec<User> = users
        .filter(id.le(tess.id))
        .union(users.filter(id.ge(tess.id)))
        .limit(2)
        .positional_order_by(1) // id is the first column
        .load(conn)
        .unwrap();

    let expected_data = vec![User::new(sean.id, "Sean"), User::new(tess.id, "Tess")];
    assert_eq!(expected_data, data);
}

#[diesel_test_helper::test]
fn union_with_offset() {
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
    let _jim = &data[2];

    let data: Vec<User> = users
        .filter(id.le(tess.id))
        .union(users.filter(id.ge(tess.id)))
        .positional_order_by(2) // name is the second column
        .limit(3)
        .offset(1)
        .load(conn)
        .unwrap();

    let expected_data = vec![User::new(sean.id, "Sean"), User::new(tess.id, "Tess")];
    assert_eq!(expected_data, data);
}

#[diesel_test_helper::test]
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
        .positional_order_by(OrderColumn::from(1).asc())
        .load::<String>(conn)
        .unwrap();

    assert_eq!(vec![String::from("Jim"), "Sean".into()], users);
}

#[diesel_test_helper::test]
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

#[diesel_test_helper::test]
fn positional_order_by() {
    use crate::schema::users::dsl::*;

    let conn = &mut connection();
    let data = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", Some("green")),
        NewUser::new("Jim", None),
        NewUser::new("Tess", Some("blue")),
        NewUser::new("Arnold", Some("red")),
    ];
    insert_into(users).values(&data).execute(conn).unwrap();
    let data = users.order(id).load::<User>(conn).unwrap();
    let sean = &data[0];
    let tess_green = &data[1];
    let jim = &data[2];
    let tess_blue = &data[3];
    let arnold = &data[4];

    let expected_data = vec![
        User::new(sean.id, "Sean"),
        User::new(jim.id, "Jim"),
        User::with_hair_color(tess_blue.id, "Tess", "blue"),
        User::with_hair_color(tess_green.id, "Tess", "green"),
        User::with_hair_color(arnold.id, "Arnold", "red"),
    ];
    let data: Vec<_> = users
        .filter(id.le(jim.id))
        .union(users.filter(id.ge(jim.id)))
        .positional_order_by((
            // hair color is the third column
            // Also, we don't need OrderColumn here because .asc() is the default direction
            #[cfg(not(feature = "postgres"))]
            3,
            // postgres doesn't sort nulls first by default, so we need to call nulls_first().
            // This also tests whether or not NullsFirst implements PositionalOrderExpr
            #[cfg(feature = "postgres")]
            OrderColumn::from(3).asc().nulls_first(), // hair color is the third column
            OrderColumn::from(2).desc(), // name is the second column
        ))
        .load(conn)
        .unwrap();
    assert_eq!(expected_data, data);
}
