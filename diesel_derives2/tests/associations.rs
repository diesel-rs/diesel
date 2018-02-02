use diesel::*;
use helpers::*;

type Backend = <TestConnection as Connection>::Backend;

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
    #[belongs_to(User)]
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
    #[belongs_to(User, foreign_key = "belongs_to_user")]
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
    #[belongs_to(Tree, foreign_key = "parent_id")]
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
