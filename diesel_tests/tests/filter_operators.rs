use schema::*;
use diesel::*;

#[test]
fn filter_by_inequality() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(vec![tess.clone()], users.filter(name.ne("Sean")).load(&connection).unwrap());
    assert_eq!(vec![sean.clone()], users.filter(name.ne("Tess")).load(&connection).unwrap());
    assert_eq!(vec![sean, tess], users.filter(name.ne("Jim")).load(&connection).unwrap());
}

#[test]
fn filter_by_gt() {
    use schema::users::dsl::*;

    let connection = connection_with_3_users();
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    assert_eq!(vec![tess, jim.clone()],
        users.filter(id.gt(1)).load(&connection).unwrap());
    assert_eq!(vec![jim], users.filter(id.gt(2)).load(&connection).unwrap());
}

#[test]
fn filter_by_ge() {
    use schema::users::dsl::*;

    let connection = connection_with_3_users();
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    assert_eq!(vec![tess, jim.clone()],
        users.filter(id.ge(2)).load(&connection).unwrap());
    assert_eq!(vec![jim], users.filter(id.ge(3)).load(&connection).unwrap());
}

#[test]
fn filter_by_lt() {
    use schema::users::dsl::*;

    let connection = connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(vec![sean.clone(), tess],
        users.filter(id.lt(3)).load(&connection).unwrap());
    assert_eq!(vec![sean], users.filter(id.lt(2)).load(&connection).unwrap());
}

#[test]
fn filter_by_le() {
    use schema::users::dsl::*;

    let connection = connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(vec![sean.clone(), tess],
        users.filter(id.le(2)).load(&connection).unwrap());
    assert_eq!(vec![sean], users.filter(id.le(1)).load(&connection).unwrap());
}

#[test]
fn filter_by_between() {
    use schema::users::dsl::*;

    let connection = connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    assert_eq!(vec![sean, tess.clone(), jim.clone()],
        users.filter(id.between(1..3)).load(&connection).unwrap());
    assert_eq!(vec![tess, jim],
        users.filter(id.between(2..3)).load(&connection).unwrap());
}

#[test]
fn filter_by_like() {
    use schema::users::dsl::*;

    let connection = connection();
    let data = vec![
        NewUser::new("Sean Griffin", None),
        NewUser::new("Tess Griffin", None),
        NewUser::new("Jim", None),
    ];
    batch_insert(&data, users, &connection);
    let data = users.load::<User>(&connection).unwrap();
    let sean = data[0].clone();
    let tess = data[1].clone();
    let jim = data[2].clone();

    assert_eq!(vec![sean, tess],
        users.filter(name.like("%Griffin")).load(&connection).unwrap());
    assert_eq!(vec![jim],
        users.filter(name.not_like("%Griffin")).load(&connection).unwrap());
}

#[test]
#[cfg(feature = "postgres")]
fn filter_by_any() {
    use schema::users::dsl::*;
    use diesel::expression::dsl::any;

    let connection = connection_with_3_users();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let jim = User::new(3, "Jim");

    let owned_names = vec!["Sean", "Tess"];
    let borrowed_names: &[&str] = &["Sean", "Jim"];
    assert_eq!(vec![sean.clone(), tess],
        users.filter(name.eq(any(owned_names))).load(&connection).unwrap());
    assert_eq!(vec![sean, jim],
        users.filter(name.eq(any(borrowed_names))).load(&connection).unwrap());
}

fn connection_with_3_users() -> TestConnection {
    let connection = connection_with_sean_and_tess_in_users_table();
    connection.execute("INSERT INTO users (id, name) VALUES (3, 'Jim')").unwrap();
    connection
}
