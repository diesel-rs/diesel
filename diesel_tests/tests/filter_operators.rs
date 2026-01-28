use crate::schema::*;
use diesel::*;

#[diesel_test_helper::test]
fn filter_by_inequality() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![tess.clone()],
        users.filter(name.ne("Sean")).load(connection).unwrap()
    );
    assert_eq!(
        vec![sean.clone()],
        users.filter(name.ne("Tess")).load(connection).unwrap()
    );
    assert_eq!(
        vec![sean, tess],
        users
            .filter(name.ne("Jim"))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
}

#[diesel_test_helper::test]
fn filter_by_gt() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    assert_eq!(
        vec![tess, jim.clone()],
        users
            .filter(id.gt(1))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(vec![jim], users.filter(id.gt(2)).load(connection).unwrap());
}

#[diesel_test_helper::test]
fn filter_by_ge() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    assert_eq!(
        vec![tess, jim.clone()],
        users
            .filter(id.ge(2))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(vec![jim], users.filter(id.ge(3)).load(connection).unwrap());
}

#[diesel_test_helper::test]
fn filter_by_lt() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![sean.clone(), tess],
        users
            .filter(id.lt(3))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(vec![sean], users.filter(id.lt(2)).load(connection).unwrap());
}

#[diesel_test_helper::test]
fn filter_by_le() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(
        vec![sean.clone(), tess],
        users
            .filter(id.le(2))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(vec![sean], users.filter(id.le(1)).load(connection).unwrap());
}

#[diesel_test_helper::test]
fn filter_by_between() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    assert_eq!(
        vec![sean, tess.clone(), jim.clone()],
        users
            .filter(id.between(1, 3))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(
        vec![tess, jim],
        users
            .filter(id.between(2, 3))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
}

#[diesel_test_helper::test]
fn filter_by_like() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    let data = vec![
        NewUser::new("Sean Griffin", None),
        NewUser::new("Tess Griffin", None),
        NewUser::new("Jim", None),
    ];
    insert_into(users)
        .values(&data)
        .execute(connection)
        .unwrap();
    let data = users.load::<User>(connection).unwrap();
    let sean = data[0].clone();
    let tess = data[1].clone();
    let jim = data[2].clone();

    assert_eq!(
        vec![sean, tess],
        users
            .filter(name.like("%Griffin"))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(
        vec![jim],
        users
            .filter(name.not_like("%Griffin"))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_by_ilike() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    let data = vec![
        NewUser::new("Sean Griffin", None),
        NewUser::new("Tess Griffin", None),
        NewUser::new("Jim", None),
    ];
    insert_into(users)
        .values(&data)
        .execute(connection)
        .unwrap();
    let data = users.load::<User>(connection).unwrap();
    let sean = data[0].clone();
    let tess = data[1].clone();
    let jim = data[2].clone();

    assert_eq!(
        vec![sean, tess],
        users
            .filter(name.ilike("%grifFin"))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(
        vec![jim],
        users
            .filter(name.not_ilike("%grifFin"))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_by_any() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    let owned_names = vec!["Sean", "Tess"];
    let borrowed_names: &[&str] = &["Sean", "Jim"];
    assert_eq!(
        vec![sean.clone(), tess],
        users
            .filter(name.eq_any(owned_names))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(
        vec![sean, jim],
        users
            .filter(name.eq_any(borrowed_names))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
}

#[diesel_test_helper::test]
fn filter_by_in() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    let owned_names = vec!["Sean", "Tess"];
    let borrowed_names: &[&str] = &["Sean", "Jim"];
    assert_eq!(
        vec![sean.clone(), tess],
        users
            .filter(name.eq_any(owned_names))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
    assert_eq!(
        vec![sean, jim],
        users
            .filter(name.eq_any(borrowed_names))
            .order(id.asc())
            .load(connection)
            .unwrap()
    );
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_by_in_explicit_array() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    let users_alias = alias!(crate::schema::users as users_alias);

    let query_subselect = users
        .filter(name.eq_any(dsl::array(users_alias.select(users_alias.field(name)))))
        .order_by(id);

    let debug_subselect: String = debug_query::<diesel::pg::Pg, _>(&query_subselect).to_string();
    if !debug_subselect
        .contains(r#"= ANY(ARRAY(SELECT "users_alias"."name" FROM "users" AS "users_alias"))"#)
    {
        panic!("Generated query (subselect) does not contain expected SQL: {debug_subselect}");
    }

    assert_eq!(
        &[sean.clone(), tess.clone(), jim] as &[_],
        query_subselect.load(connection).unwrap()
    );

    let query_array_construct = users
        .filter(
            name.nullable().eq_any(dsl::array((
                users_alias
                    .filter(users_alias.field(id).eq(1))
                    .select(users_alias.field(name))
                    .single_value(),
                "Tess",
                None::<&str>,
            ))),
        )
        .order_by(id);

    let debug_array_construct: String =
        debug_query::<diesel::pg::Pg, _>(&query_array_construct).to_string();
    if !debug_array_construct.contains("= ANY(ARRAY[(SELECT") {
        panic!(
            "Generated query (array construct) does not contain expected SQL: {debug_array_construct}"
        );
    }

    assert_eq!(
        &[sean, tess] as &[_],
        query_array_construct.load(connection).unwrap()
    );
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_array_by_in() {
    use crate::schema::posts::dsl::*;

    let connection: &mut PgConnection = &mut connection();
    let tag_combinations_to_look_for: &[&[&str]] = &[&["foo"], &["foo", "bar"], &["baz"]];
    let result: Vec<i32> = posts
        .filter(tags.eq_any(tag_combinations_to_look_for))
        .select(id)
        .load(connection)
        .unwrap();
    assert_eq!(result, &[] as &[i32]);
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn filter_array_by_not_in() {
    use crate::schema::posts::dsl::*;

    let connection: &mut PgConnection = &mut connection();
    let tag_combinations_to_look_for: &[&[&str]] = &[&["foo"], &["foo", "bar"], &["baz"]];
    let result: Vec<i32> = posts
        .filter(tags.ne_all(tag_combinations_to_look_for))
        .select(id)
        .load(connection)
        .unwrap();
    assert_eq!(result, &[] as &[i32]);
}

fn connection_with_3_users() -> TestConnection {
    let mut connection = connection_with_sean_and_tess_in_users_table();
    diesel::sql_query("INSERT INTO users (id, name) VALUES (3, 'Jim')")
        .execute(&mut connection)
        .unwrap();
    connection
}
