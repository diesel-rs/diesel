//! Generated code must not depend on what the name `diesel` means where the
//! macro was invoked.
//!
//! Every module below shadows that name with an empty module, so anything the
//! expansion reaches through an unrooted `diesel::` path fails to resolve.

use crate::helpers::TestBackend;
use diesel::prelude::*;

mod shadowed_schema {
    mod diesel {}

    ::diesel::table! {
        hygiene_users (id) {
            id -> ::diesel::sql_types::Integer,
            name -> ::diesel::sql_types::Text,
        }
    }

    ::diesel::table! {
        hygiene_posts (id) {
            id -> ::diesel::sql_types::Integer,
            user_id -> ::diesel::sql_types::Integer,
        }
    }

    ::diesel::joinable!(hygiene_posts -> hygiene_users (user_id));
    ::diesel::allow_tables_to_appear_in_same_query!(hygiene_users, hygiene_posts);

    #[derive(
        ::diesel::Insertable,
        ::diesel::Queryable,
        ::diesel::Selectable,
        ::diesel::QueryableByName,
        ::diesel::AsChangeset,
        ::diesel::Identifiable,
    )]
    #[diesel(table_name = hygiene_users)]
    pub struct User {
        pub id: i32,
        pub name: String,
    }

    #[derive(::diesel::HasQuery, ::diesel::Insertable)]
    #[diesel(table_name = hygiene_users)]
    pub struct UserQuery {
        pub id: i32,
        pub name: String,
    }

    #[derive(::diesel::Identifiable, ::diesel::Associations, ::diesel::Queryable)]
    #[diesel(table_name = hygiene_posts, belongs_to(User))]
    pub struct Post {
        pub id: i32,
        pub user_id: i32,
    }

    #[derive(Debug, ::diesel::expression::AsExpression, ::diesel::deserialize::FromSqlRow)]
    #[diesel(sql_type = ::diesel::sql_types::Text)]
    pub struct Name(pub String);

    #[derive(::diesel::sql_types::SqlType)]
    pub struct MySqlType;

    #[derive(
        ::diesel::query_builder::QueryId,
        ::diesel::sql_types::DieselNumericOps,
        ::diesel::expression::ValidGrouping,
    )]
    pub struct Wrapper<T> {
        pub inner: T,
    }

    impl<T: ::diesel::expression::Expression> ::diesel::expression::Expression for Wrapper<T> {
        type SqlType = T::SqlType;
    }

    impl<T, DB> ::diesel::query_builder::QueryFragment<DB> for Wrapper<T>
    where
        DB: ::diesel::backend::Backend,
        T: ::diesel::query_builder::QueryFragment<DB>,
    {
        fn walk_ast<'b>(
            &'b self,
            pass: ::diesel::query_builder::AstPass<'_, 'b, DB>,
        ) -> ::diesel::QueryResult<()> {
            self.inner.walk_ast(pass)
        }
    }
}

mod shadowed_functions {
    mod diesel {}

    ::diesel::define_sql_function!(
        fn hygiene_lower(x: ::diesel::sql_types::Text) -> ::diesel::sql_types::Text
    );

    #[::diesel::declare_sql_function]
    extern "SQL" {
        fn hygiene_upper(x: ::diesel::sql_types::Text) -> ::diesel::sql_types::Text;
    }
}

mod shadowed_auto_type {
    mod diesel {}

    use super::shadowed_schema::hygiene_users;
    use ::diesel::prelude::*;

    #[::diesel::dsl::auto_type]
    pub fn named_user() -> _ {
        hygiene_users::table.filter(hygiene_users::id.eq(1_i32))
    }
}

mod shadowed_connection {
    mod diesel {}

    #[derive(::diesel::MultiConnection)]
    pub enum AnyConnection {
        #[cfg(feature = "postgres")]
        Pg(::diesel::PgConnection),
        #[cfg(feature = "sqlite")]
        Sqlite(::diesel::SqliteConnection),
        #[cfg(feature = "mysql")]
        Mysql(::diesel::MysqlConnection),
        #[cfg(feature = "mariadb")]
        Mariadb(::diesel::MariadbConnection),
    }
}

/// The derives beside a shadowed `diesel` build the statements they promise.
#[test]
fn derives_survive_a_shadowed_diesel_name() {
    let user = shadowed_schema::User {
        id: 1,
        name: "sean".into(),
    };

    let insert = diesel::insert_into(shadowed_schema::hygiene_users::table).values(&user);
    let sql = diesel::debug_query::<TestBackend, _>(&insert).to_string();
    assert!(sql.contains("hygiene_users"), "{sql}");

    let posts = shadowed_schema::Post::belonging_to(&user);
    let sql = diesel::debug_query::<TestBackend, _>(&posts).to_string();
    assert!(sql.contains("hygiene_posts"), "{sql}");

    let base = <shadowed_schema::UserQuery as diesel::HasQuery<TestBackend>>::base_query();
    let sql = diesel::debug_query::<TestBackend, _>(&base).to_string();
    assert!(sql.contains("hygiene_users"), "{sql}");

    let insert = diesel::insert_into(shadowed_schema::hygiene_users::table).values(
        shadowed_schema::UserQuery {
            id: 2,
            name: "tess".into(),
        },
    );
    let sql = diesel::debug_query::<TestBackend, _>(&insert).to_string();
    assert!(sql.contains("hygiene_users"), "{sql}");

    fn assert_connection<C: diesel::Connection>() {}
    assert_connection::<shadowed_connection::AnyConnection>();

    fn assert_sql_type<T: diesel::sql_types::SqlType>() {}
    assert_sql_type::<shadowed_schema::MySqlType>();

    let sum = shadowed_schema::Wrapper {
        inner: shadowed_schema::hygiene_users::id,
    } + 1;
    let sql = diesel::debug_query::<TestBackend, _>(&sum).to_string();
    assert!(sql.contains("id"), "{sql}");
}

/// A function declared beside a shadowed `diesel` renders as a call.
#[test]
fn sql_functions_survive_a_shadowed_diesel_name() {
    use shadowed_functions::{hygiene_lower, hygiene_upper};

    let lowered = hygiene_lower(shadowed_schema::hygiene_users::name);
    let sql = diesel::debug_query::<TestBackend, _>(&lowered).to_string();
    assert!(sql.contains("hygiene_lower("), "{sql}");

    let uppered = hygiene_upper(shadowed_schema::hygiene_users::name);
    let sql = diesel::debug_query::<TestBackend, _>(&uppered).to_string();
    assert!(sql.contains("hygiene_upper("), "{sql}");
}

/// An `#[auto_type]` function beside a shadowed `diesel` infers its return type.
#[test]
fn auto_type_survives_a_shadowed_diesel_name() {
    let query = shadowed_auto_type::named_user();
    let sql = diesel::debug_query::<TestBackend, _>(&query).to_string();
    assert!(sql.contains("hygiene_users"), "{sql}");
}
