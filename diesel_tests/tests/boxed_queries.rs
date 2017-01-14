use super::schema::*;
use diesel::*;

#[test]
fn boxed_queries_can_be_executed() {
    let connection = connection_with_sean_and_tess_in_users_table();
    insert(&NewUser::new("Jim", None, UserType::Default)).into(users::table)
        .execute(&connection).unwrap();
    let query_which_fails_unless_all_segments_are_applied =
        users::table
            .select(users::name)
            .filter(users::name.ne("jim"))
            .order(users::name.desc())
            .limit(1)
            .offset(1)
            .into_boxed();

    let expected_data = vec!["Sean".to_string()];
    let data = query_which_fails_unless_all_segments_are_applied.load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn boxed_queries_can_differ_conditionally() {
    let connection = connection_with_sean_and_tess_in_users_table();
    insert(&NewUser::new("Jim", None, UserType::Default)).into(users::table)
        .execute(&connection).unwrap();

    enum Query { All, Ordered, One };
    let source = |query| {
        match query {
            Query::All => users::table.into_boxed(),
            Query::Ordered =>
                users::table
                    .order(users::name.desc())
                    .into_boxed(),
            Query::One =>
                users::table
                    .filter(users::name.ne("jim"))
                    .order(users::name.desc())
                    .limit(1)
                    .offset(1)
                    .into_boxed(),
        }
    };
    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);
    let jim = find_user_by_name("Jim", &connection);

    let all = source(Query::All).load(&connection);
    let expected_data = vec![sean.clone(), tess.clone(), jim.clone()];
    assert_eq!(Ok(expected_data), all);

    let ordered = source(Query::Ordered).load(&connection);
    let expected_data = vec![tess.clone(), sean.clone(), jim.clone()];
    assert_eq!(Ok(expected_data), ordered);

    let one = source(Query::One).load(&connection);
    let expected_data = vec![sean.clone()];
    assert_eq!(Ok(expected_data), one);
}

#[test]
fn boxed_queries_implement_select_dsl() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let data = users::table.into_boxed().select(users::name)
        .load::<String>(&connection);
    assert_eq!(Ok(vec!["Sean".into(), "Tess".into()]), data);
}

#[test]
fn boxed_queries_implement_filter_dsl() {
    let connection = connection_with_sean_and_tess_in_users_table();
    insert(&NewUser::new("Shane", None, UserType::Default)).into(users::table)
        .execute(&connection).unwrap();
    let data = users::table.into_boxed()
        .select(users::name)
        .filter(users::name.ne("Sean"))
        .filter(users::name.like("S%"))
        .load(&connection);
    assert_eq!(Ok(vec![String::from("Shane")]), data);
}

#[test]
fn boxed_queries_implement_limit_dsl() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let data = users::table.into_boxed().limit(1).load(&connection);
    let expected_data = vec![find_user_by_name("Sean", &connection)];
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn boxed_queries_implement_offset_dsl() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let data = users::table.into_boxed()
        .limit(1)
        .offset(1)
        .load(&connection);
    let expected_data = vec![find_user_by_name("Tess", &connection)];
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn boxed_queries_implement_order_dsl() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let data = users::table.into_boxed().order(users::name.desc())
        .load(&connection);
    let expected_data = vec![
        find_user_by_name("Tess", &connection),
        find_user_by_name("Sean", &connection),
    ];
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn boxed_queries_can_use_borrowed_data() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let s = String::from("Sean");
    let data = users::table.into_boxed()
        .filter(users::name.eq(&s))
        .load(&connection);
    let expected_data = vec![find_user_by_name("Sean", &connection)];
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn queries_with_borrowed_data_can_be_boxed() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let s = String::from("Tess");
    let data = users::table
        .filter(users::name.eq(&s))
        .into_boxed()
        .load(&connection);
    let expected_data = vec![find_user_by_name("Tess", &connection)];
    assert_eq!(Ok(expected_data), data);
}
