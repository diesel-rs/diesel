use crate::helpers::*;
use diesel::*;

type Backend = <TestConnection as Connection>::Backend;

// https://github.com/rust-lang/rust/issues/124396
#[allow(unknown_lints, non_local_definitions)]
#[test]
fn simple_belongs_to() {
    table! {
        users {
            id -> Integer,
            name -> Text,
        }
    }

    table! {
        posts {
            id -> Integer,
            user_id -> Integer,
            title -> Text,
        }
    }

    allow_tables_to_appear_in_same_query!(users, posts);

    #[derive(Identifiable)]
    pub struct User {
        id: i32,
    }

    #[derive(Associations, Identifiable)]
    #[diesel(belongs_to(User))]
    pub struct Post {
        id: i32,
        user_id: i32,
    }

    joinable!(posts -> users(user_id));

    let _can_join_tables = posts::table
        .inner_join(users::table)
        .select((users::id, users::name, posts::id))
        .filter(
            posts::id
                .eq(1)
                .and(posts::user_id.eq(2))
                .and(posts::title.eq("Bar")),
        );

    let _can_reverse_join_tables = users::table
        .inner_join(posts::table)
        .select((posts::id, posts::user_id, posts::title))
        .filter(users::id.eq(1).and(users::name.eq("Sean")));

    let t = User { id: 42 };

    let belong_to = Post::belonging_to(&t);
    let filter = posts::table.filter(posts::user_id.eq(42));

    assert_eq!(
        debug_query::<Backend, _>(&belong_to).to_string(),
        debug_query::<Backend, _>(&filter).to_string()
    );
}

#[test]
fn table_in_different_module() {
    mod schema {
        table! {
            users {
                id -> Integer,
                name -> Text,
            }
        }

        table! {
            posts {
                id -> Integer,
                user_id -> Integer,
                title -> Text,
            }
        }

        allow_tables_to_appear_in_same_query!(users, posts);

        joinable!(posts -> users(user_id));
    }

    #[derive(Identifiable)]
    #[diesel(table_name = schema::users)]
    pub struct User {
        id: i32,
    }

    #[derive(Associations, Identifiable)]
    #[diesel(table_name = schema::posts)]
    #[diesel(belongs_to(User))]
    pub struct Post {
        id: i32,
        user_id: i32,
    }

    let _can_join_tables = schema::posts::table
        .inner_join(schema::users::table)
        .select((schema::users::id, schema::users::name, schema::posts::id))
        .filter(
            schema::posts::id
                .eq(1)
                .and(schema::posts::user_id.eq(2))
                .and(schema::posts::title.eq("Bar")),
        );

    let _can_reverse_join_tables = schema::users::table
        .inner_join(schema::posts::table)
        .select((
            schema::posts::id,
            schema::posts::user_id,
            schema::posts::title,
        ))
        .filter(schema::users::id.eq(1).and(schema::users::name.eq("Sean")));

    let t = User { id: 42 };

    let belong_to = Post::belonging_to(&t);
    let filter = schema::posts::table.filter(schema::posts::user_id.eq(42));

    assert_eq!(
        debug_query::<Backend, _>(&belong_to).to_string(),
        debug_query::<Backend, _>(&filter).to_string()
    );
}

// https://github.com/rust-lang/rust/issues/124396
#[allow(unknown_lints, non_local_definitions)]
#[test]
fn custom_foreign_key() {
    table! {
        users {
            id -> Integer,
            name -> Text,
        }
    }

    table! {
        posts {
            id -> Integer,
            belongs_to_user -> Integer,
            title -> Text,
        }
    }

    allow_tables_to_appear_in_same_query!(users, posts);

    #[derive(Identifiable)]
    pub struct User {
        id: i32,
    }

    #[derive(Associations, Identifiable)]
    #[diesel(belongs_to(User, foreign_key = belongs_to_user))]
    pub struct Post {
        id: i32,
        belongs_to_user: i32,
    }

    joinable!(posts -> users(belongs_to_user));

    let _can_join_tables = posts::table
        .inner_join(users::table)
        .select((users::id, users::name))
        .filter(
            posts::id
                .eq(1)
                .and(posts::belongs_to_user.eq(2))
                .and(posts::title.eq("Bar")),
        );

    let _can_reverse_join_tables = users::table
        .inner_join(posts::table)
        .select((posts::id, posts::belongs_to_user, posts::title))
        .filter(users::id.eq(1).and(users::name.eq("Sean")));

    let t = User { id: 42 };

    let belong_to = Post::belonging_to(&t);
    let filter = posts::table.filter(posts::belongs_to_user.eq(42));

    assert_eq!(
        debug_query::<Backend, _>(&belong_to).to_string(),
        debug_query::<Backend, _>(&filter).to_string()
    );
}

#[test]
fn self_referential() {
    table! {
        trees {
            id -> Integer,
            parent_id -> Nullable<Integer>,
        }
    }

    #[derive(Associations, Identifiable)]
    #[diesel(belongs_to(Tree, foreign_key = parent_id))]
    pub struct Tree {
        id: i32,
        parent_id: Option<i32>,
    }
    let t = Tree {
        id: 42,
        parent_id: None,
    };

    let belong_to = Tree::belonging_to(&t);
    let filter = trees::table.filter(trees::parent_id.eq(42));
    assert_eq!(
        debug_query::<Backend, _>(&belong_to).to_string(),
        debug_query::<Backend, _>(&filter).to_string()
    );
}

#[test]
fn multiple_associations() {
    table! {
        users {
            id -> Integer,
        }
    }

    table! {
        posts {
            id -> Integer,
        }
    }

    table! {
        comments {
            id -> Integer,
            user_id -> Integer,
            post_id -> Integer,
        }
    }

    #[derive(Identifiable)]
    struct User {
        id: i32,
    }

    #[derive(Identifiable)]
    struct Post {
        id: i32,
    }

    #[derive(Identifiable, Associations)]
    #[diesel(belongs_to(User))]
    #[diesel(belongs_to(Post))]
    struct Comment {
        id: i32,
        user_id: i32,
        post_id: i32,
    }

    let user = User { id: 1 };
    let post = Post { id: 2 };

    let query = Comment::belonging_to(&user);
    let expected = comments::table.filter(comments::user_id.eq(1));
    assert_eq!(
        debug_query::<Backend, _>(&query).to_string(),
        debug_query::<Backend, _>(&expected).to_string()
    );
    let query = Comment::belonging_to(&post);
    let expected = comments::table.filter(comments::post_id.eq(2));
    assert_eq!(
        debug_query::<Backend, _>(&query).to_string(),
        debug_query::<Backend, _>(&expected).to_string()
    );
}

#[test]
fn foreign_key_field_with_column_rename() {
    table! {
        users {
            id -> Integer,
        }
    }

    table! {
        posts {
            id -> Integer,
            user_id -> Integer,
        }
    }

    #[derive(Identifiable, Clone, Copy)]
    pub struct User {
        id: i32,
    }

    #[derive(Associations, Identifiable, Clone, Copy, PartialEq, Debug, Eq)]
    #[diesel(belongs_to(User))]
    pub struct Post {
        id: i32,
        #[diesel(column_name = user_id)]
        author_id: i32,
    }

    let user1 = User { id: 1 };
    let user2 = User { id: 2 };
    let post1 = Post {
        id: 1,
        author_id: 2,
    };
    let post2 = Post {
        id: 2,
        author_id: 1,
    };

    let query = Post::belonging_to(&user1);
    let expected = posts::table.filter(posts::user_id.eq(1));
    assert_eq!(
        debug_query::<Backend, _>(&query).to_string(),
        debug_query::<Backend, _>(&expected).to_string()
    );

    let users = vec![user1, user2];
    let posts = vec![post1, post2].grouped_by(&users);
    assert_eq!(vec![vec![post2], vec![post1]], posts);
}

#[test]
fn tuple_struct() {
    table! {
        users {
            id -> Integer,
        }
    }

    table! {
        posts {
            id -> Integer,
            user_id -> Integer,
        }
    }

    #[derive(Identifiable)]
    pub struct User {
        id: i32,
    }

    #[derive(Associations, Identifiable)]
    #[diesel(belongs_to(User))]
    pub struct Post(
        #[diesel(column_name = id)] i32,
        #[diesel(column_name = user_id)] i32,
    );

    let user = User { id: 1 };

    let query = Post::belonging_to(&user);
    let expected = posts::table.filter(posts::user_id.eq(1));
    assert_eq!(
        debug_query::<Backend, _>(&query).to_string(),
        debug_query::<Backend, _>(&expected).to_string()
    );
}
