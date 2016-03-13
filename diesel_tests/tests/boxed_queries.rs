use super::schema::*;
use diesel::*;
use diesel::query_builder::BoxedSelectStatement;

#[test]
fn boxed_queries_can_be_executed() {
    let connection = connection_with_sean_and_tess_in_users_table();
    insert(&NewUser::new("Jim", None)).into(users::table)
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
    insert(&NewUser::new("Jim", None)).into(users::table)
        .execute(&connection).unwrap();

    enum Query { All, Ordered, One };
    fn source(query: Query)
        -> BoxedSelectStatement<users::SqlType, users::table, TestBackend>
    {
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
    }
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
