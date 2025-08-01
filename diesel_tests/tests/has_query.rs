use super::schema::*;
use diesel::*;

#[diesel_test_helper::test]
fn selecting_basic_data() {
    use crate::schema::users;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    #[derive(HasQuery, PartialEq, Debug)]
    pub struct User {
        pub id: i32,
        pub name: String,
        pub hair_color: Option<String>,
    }

    let expected_data = vec![
        User {
            id: 1,
            name: "Sean".to_string(),
            hair_color: None,
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    let actual_data = User::query().order(users::id).load(connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn selecting_custom_base_query() {
    use crate::schema::{posts, users};

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    #[derive(HasQuery, PartialEq, Debug)]
    pub struct Post {
        id: i32,
        title: String,
        user_id: i32,
    }

    #[derive(HasQuery, PartialEq, Debug)]
    #[diesel(base_query = users::table.left_join(posts::table))]
    pub struct User {
        pub id: i32,
        pub name: String,
        pub hair_color: Option<String>,
        #[diesel(embed)]
        post: Option<Post>,
    }

    let expected_data: Vec<User> = vec![
        User {
            id: 1,
            name: "Sean".to_string(),
            hair_color: None,
            post: None,
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: None,
            post: None,
        },
    ];
    let actual_data = User::query().order(users::id).load(connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn fully_custom() {
    use crate::schema::{posts, users};

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    #[derive(HasQuery, PartialEq, Debug)]
    #[diesel(base_query = users::table.left_join(posts::table).group_by(users::id))]
    pub struct User {
        pub id: i32,
        pub name: String,
        pub hair_color: Option<String>,
        #[diesel(select_expression = diesel::dsl::count(posts::id.nullable()))]
        post_count: i64,
    }

    let expected_data = vec![
        User {
            id: 1,
            name: "Sean".to_string(),
            hair_color: None,
            post_count: 0,
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: None,
            post_count: 0,
        },
    ];
    let actual_data = User::query().order(users::id).load(connection).unwrap();
    assert_eq!(expected_data, actual_data);
}
