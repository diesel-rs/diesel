#![allow(dead_code, unused_must_use)]

#[test]
fn simple_belongs_to() {
    use diesel::*;

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

    #[derive(Identifiable)]
    pub struct User {
        id: i32,
        name: String
    }

    #[derive(Associations)]
    #[belongs_to(User)]
    pub struct Post {
        id: i32,
        user_id: i32,
        title: String,
    }


    posts::table.inner_join(users::table)
        .select((users::id, users::name))
        .filter(posts::id.eq(1)
            .and(posts::user_id.eq(2))
                .and(posts::title.eq("Bar")));

    users::table.inner_join(posts::table)
        .select((posts::id, posts::user_id, posts::title))
        .filter(users::id.eq(1)
            .and(users::name.eq("Sean")));
}


#[test]
fn custom_foreign_key() {
    use diesel::*;

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

    #[derive(Identifiable)]
    pub struct User {
        id: i32,
        name: String
    }

    #[derive(Associations)]
    #[belongs_to(User, foreign_key = "belongs_to_user")]
    pub struct Post {
        id: i32,
        belongs_to_user: i32,
        title: String,
    }


    posts::table.inner_join(users::table)
        .select((users::id, users::name))
        .filter(posts::id.eq(1)
            .and(posts::belongs_to_user.eq(2))
                .and(posts::title.eq("Bar")));

    users::table.inner_join(posts::table)
        .select((posts::id, posts::belongs_to_user, posts::title))
        .filter(users::id.eq(1)
            .and(users::name.eq("Sean")));
}

#[test]
fn self_referential() {
    use diesel::*;

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

    Tree::belonging_to(&Tree{ id: 42, parent_id: None });
}
