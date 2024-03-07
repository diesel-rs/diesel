pub(crate) mod prelude {
    #[cfg_attr(
        any(feature = "huge-tables", feature = "large-tables"),
        allow(deprecated)
    )]
    // This is a false positive, we reexport it later
    #[allow(unreachable_pub, unused_imports)]
    #[doc(inline)]
    pub use crate::{
        allow_columns_to_appear_in_same_group_by_clause, allow_tables_to_appear_in_same_query,
        joinable, table,
    };
}

#[doc(inline)]
pub use diesel_derives::table_proc as table;

/// Allow two tables to be referenced in a join query without providing an
/// explicit `ON` clause.
///
/// The generated `ON` clause will always join to the primary key of the parent
/// table. This macro removes the need to call [`.on`] explicitly, you will
/// still need to invoke
/// [`allow_tables_to_appear_in_same_query!`](crate::allow_tables_to_appear_in_same_query)
/// for these two tables to be able to use the resulting query, unless you are
/// using `diesel print-schema` which will generate it for you.
///
/// If you are using `diesel print-schema`, an invocation of this macro
/// will be generated for every foreign key in your database unless
/// one of the following is true:
///
/// - The foreign key references something other than the primary key
/// - The foreign key is composite
/// - There is more than one foreign key connecting two tables
/// - The foreign key is self-referential
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use schema::*;
///
/// # /*
/// joinable!(posts -> users (user_id));
/// allow_tables_to_appear_in_same_query!(posts, users);
/// # */
///
/// # fn main() {
/// let implicit_on_clause = users::table.inner_join(posts::table);
/// let implicit_on_clause_sql = diesel::debug_query::<DB, _>(&implicit_on_clause).to_string();
///
/// let explicit_on_clause = users::table
///     .inner_join(posts::table.on(posts::user_id.eq(users::id)));
/// let explicit_on_clause_sql = diesel::debug_query::<DB, _>(&explicit_on_clause).to_string();
///
/// assert_eq!(implicit_on_clause_sql, explicit_on_clause_sql);
/// # }
///
/// ```
///
/// In the example above, the line `joinable!(posts -> users (user_id));`
///
/// specifies the relation of the tables and the ON clause in the following way:
///
/// `child_table -> parent_table (foreign_key)`
///
/// * `parent_table` is the Table with the Primary key.
///
/// * `child_table` is the Table with the Foreign key.
///
/// So given the Table declaration from [Associations docs](crate::associations)
///
/// * The parent table would be `User`
/// * The child table would be `Post`
/// * and the Foreign key would be `Post.user_id`
///
/// For joins that do not explicitly use on clauses via [`JoinOnDsl`](crate::prelude::JoinOnDsl)
/// the following on clause is generated implicitly:
/// ```sql
/// post JOIN users ON posts.user_id = users.id
/// ```
#[macro_export]
macro_rules! joinable {
    ($($child:ident)::* -> $($parent:ident)::* ($source:ident)) => {
        $crate::joinable_inner!($($child)::* ::table => $($parent)::* ::table : ($($child)::* ::$source = $($parent)::* ::table));
        $crate::joinable_inner!($($parent)::* ::table => $($child)::* ::table : ($($child)::* ::$source = $($parent)::* ::table));
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! joinable_inner {
    ($left_table:path => $right_table:path : ($foreign_key:path = $parent_table:path)) => {
        $crate::joinable_inner!(
            left_table_ty = $left_table,
            right_table_ty = $right_table,
            right_table_expr = $right_table,
            foreign_key = $foreign_key,
            primary_key_ty = <$parent_table as $crate::query_source::Table>::PrimaryKey,
            primary_key_expr =
                <$parent_table as $crate::query_source::Table>::primary_key(&$parent_table),
        );
    };

    (
        left_table_ty = $left_table_ty:ty,
        right_table_ty = $right_table_ty:ty,
        right_table_expr = $right_table_expr:expr,
        foreign_key = $foreign_key:path,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
    ) => {
        impl $crate::JoinTo<$right_table_ty> for $left_table_ty {
            type FromClause = $right_table_ty;
            type OnClause = $crate::dsl::Eq<
                $crate::internal::table_macro::NullableExpression<$foreign_key>,
                $crate::internal::table_macro::NullableExpression<$primary_key_ty>,
            >;

            fn join_target(rhs: $right_table_ty) -> (Self::FromClause, Self::OnClause) {
                use $crate::{ExpressionMethods, NullableExpressionMethods};

                (
                    rhs,
                    $foreign_key.nullable().eq($primary_key_expr.nullable()),
                )
            }
        }
    };
}

/// Allow two or more tables which are otherwise unrelated to be used together
/// in a query.
///
/// This macro must be invoked any time two tables need to appear in the same
/// query either because they are being joined together, or because one appears
/// in a subselect. When this macro is invoked with more than 2 tables, every
/// combination of those tables will be allowed to appear together.
///
/// If you are using `diesel print-schema`, an invocation of
/// this macro will be generated for you for all tables in your schema.
///
/// # Example
///
/// ```
/// # use diesel::{allow_tables_to_appear_in_same_query, table};
/// #
/// // This would be required to do `users.inner_join(posts.inner_join(comments))`
/// allow_tables_to_appear_in_same_query!(comments, posts, users);
///
/// table! {
///     comments {
///         id -> Integer,
///         post_id -> Integer,
///         body -> VarChar,
///     }
/// }
///
/// table! {
///    posts {
///        id -> Integer,
///        user_id -> Integer,
///        title -> VarChar,
///    }
/// }
///
/// table! {
///     users {
///        id -> Integer,
///        name -> VarChar,
///     }
/// }
/// ```
///
/// When more than two tables are passed, the relevant code is generated for
/// every combination of those tables. This code would be equivalent to the
/// previous example.
///
/// ```
/// # use diesel::{allow_tables_to_appear_in_same_query, table};
/// # table! {
/// #    comments {
/// #        id -> Integer,
/// #        post_id -> Integer,
/// #        body -> VarChar,
/// #    }
/// # }
/// #
/// # table! {
/// #    posts {
/// #        id -> Integer,
/// #        user_id -> Integer,
/// #        title -> VarChar,
/// #    }
/// # }
/// #
/// # table! {
/// #     users {
/// #        id -> Integer,
/// #        name -> VarChar,
/// #     }
/// # }
/// #
/// allow_tables_to_appear_in_same_query!(comments, posts);
/// allow_tables_to_appear_in_same_query!(comments, users);
/// allow_tables_to_appear_in_same_query!(posts, users);
/// #
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! allow_tables_to_appear_in_same_query {
    ($left_mod:ident, $($right_mod:ident),+ $(,)*) => {
        $(
            impl $crate::query_source::TableNotEqual<$left_mod::table> for $right_mod::table {}
            impl $crate::query_source::TableNotEqual<$right_mod::table> for $left_mod::table {}
            $crate::__diesel_internal_backend_specific_allow_tables_to_appear_in_same_query!($left_mod, $right_mod);
        )+
        $crate::allow_tables_to_appear_in_same_query!($($right_mod,)+);
    };

    ($last_table:ident,) => {};

    () => {};
}
#[doc(hidden)]
#[macro_export]
#[cfg(feature = "postgres_backend")]
macro_rules! __diesel_internal_backend_specific_allow_tables_to_appear_in_same_query {
    ($left:ident, $right:ident) => {
        impl $crate::query_source::TableNotEqual<$left::table>
            for $crate::query_builder::Only<$right::table>
        {
        }
        impl $crate::query_source::TableNotEqual<$right::table>
            for $crate::query_builder::Only<$left::table>
        {
        }
        impl $crate::query_source::TableNotEqual<$crate::query_builder::Only<$left::table>>
            for $right::table
        {
        }
        impl $crate::query_source::TableNotEqual<$crate::query_builder::Only<$right::table>>
            for $left::table
        {
        }
        impl<TSM> $crate::query_source::TableNotEqual<$left::table>
            for $crate::query_builder::Tablesample<$right::table, TSM>
        where
            TSM: $crate::internal::table_macro::TablesampleMethod,
        {
        }
        impl<TSM> $crate::query_source::TableNotEqual<$right::table>
            for $crate::query_builder::Tablesample<$left::table, TSM>
        where
            TSM: $crate::internal::table_macro::TablesampleMethod,
        {
        }
        impl<TSM>
            $crate::query_source::TableNotEqual<
                $crate::query_builder::Tablesample<$left::table, TSM>,
            > for $right::table
        where
            TSM: $crate::internal::table_macro::TablesampleMethod,
        {
        }
        impl<TSM>
            $crate::query_source::TableNotEqual<
                $crate::query_builder::Tablesample<$right::table, TSM>,
            > for $left::table
        where
            TSM: $crate::internal::table_macro::TablesampleMethod,
        {
        }
    };
}
#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "postgres_backend"))]
macro_rules! __diesel_internal_backend_specific_allow_tables_to_appear_in_same_query {
    ($left:ident, $right:ident) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __diesel_impl_allow_in_same_group_by_clause {
    (
        left = [$($left_path:tt)::+],
    ) => {};
    (
        left = [$($left_path:tt)::+],
        $($right_path:tt)::+
    ) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$($left_path)+],
            right = [$($right_path)+],
            left_tbl = [],
            left_path = [],
        }
    };
    (
        left = [$($left_path:tt)::+],
        $($right_path:tt)::+,
        $($other:tt)*
    ) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$($left_path)+],
            right = [$($right_path)+],
            left_tbl = [],
            left_path = [],
        }
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$($left_path)::+],
            $($other)*
        }
    };
    (
        left = [$left_path_p1: tt  $($left_path: tt)+],
        right = [$($right_path: tt)*],
        left_tbl = [$($left_tbl:tt)?],
        left_path = [$($left_out_path:tt)*],
    ) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$($left_path)+],
            right = [$($right_path)*],
            left_tbl = [$left_path_p1],
            left_path = [$($left_out_path)* $($left_tbl)?],
        }
    };
    (
        left = [$left_col: tt],
        right = [$($right_path: tt)*],
        left_tbl = [$($left_tbl:tt)?],
        left_path = [$($left_out_path:tt)*],
    ) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$left_col],
            right = [$($right_path)*],
            left_tbl = [$($left_tbl)?],
            left_path = [$($left_out_path)*],
            right_tbl = [],
            right_path = [],
        }
    };
    (
        left = [$left_col: tt ],
        right = [$right_path_p1: tt  $($right_path: tt)+],
        left_tbl = [$($left_tbl:tt)?],
        left_path = [$($left_out_path:tt)*],
        right_tbl = [$($right_tbl:tt)?],
        right_path = [$($right_out_path:tt)*],
    ) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$left_col],
            right = [$($right_path)+],
            left_tbl = [$($left_tbl)?],
            left_path = [$($left_out_path)*],
            right_tbl = [$right_path_p1],
            right_path = [$($right_out_path)* $($right_tbl)?],
        }
    };
    (
        left = [$left_col: tt],
        right = [$right_col: tt],
        left_tbl = [$left_tbl:tt],
        left_path = [$($left_begin:tt)*],
        right_tbl = [$right_tbl:tt],
        right_path = [$($right_begin:tt)*],
    ) => {
        $crate::static_cond! {
            if $left_tbl != $right_tbl {
                impl $crate::expression::IsContainedInGroupBy<$($left_begin ::)* $left_tbl :: $left_col> for $($right_begin ::)* $right_tbl :: $right_col {
                    type Output = $crate::expression::is_contained_in_group_by::No;
                }

                impl $crate::expression::IsContainedInGroupBy<$($right_begin ::)* $right_tbl :: $right_col> for $($left_begin ::)* $left_tbl :: $left_col {
                    type Output = $crate::expression::is_contained_in_group_by::No;
                }
            }
        }
    };
    (
        left = [$left_col: tt],
        right = [$right_col: tt],
        left_tbl = [$($left_tbl:tt)?],
        left_path = [$($left_begin:tt)*],
        right_tbl = [$($right_tbl:tt)?],
        right_path = [$($right_begin:tt)*],
    ) => {
        impl $crate::expression::IsContainedInGroupBy<$($left_begin ::)* $($left_tbl ::)? $left_col> for $($right_begin ::)* $($right_tbl ::)? $right_col {
            type Output = $crate::expression::is_contained_in_group_by::No;
        }

        impl $crate::expression::IsContainedInGroupBy<$($right_begin ::)* $($right_tbl ::)? $right_col> for $($left_begin ::)* $($left_tbl ::)? $left_col {
            type Output = $crate::expression::is_contained_in_group_by::No;
        }
    };

}

/// Allow two or more columns which are otherwise unrelated to be used together
/// in a group by clause.
///
/// This macro must be invoked any time two columns need to appear in the same
/// group by clause. When this macro is invoked with more than 2 columns, every
/// combination of those columns will be allowed to appear together.
///
/// # Example
///
/// ```
/// # include!("../doctest_setup.rs");
/// # use crate::schema::{users, posts};
/// // This would be required
///
/// allow_columns_to_appear_in_same_group_by_clause!(users::name, posts::id, posts::title);
/// # fn main() {
/// // to do implement the following join
/// users::table.inner_join(posts::table).group_by((users::name, posts::id, posts::title))
/// # ;
/// # }
/// ```
///
/// When more than two columns are passed, the relevant code is generated for
/// every combination of those columns. This code would be equivalent to the
/// previous example.
///
/// ```
/// # include!("../doctest_setup.rs");
/// # use crate::schema::{users, posts};
/// #
/// allow_columns_to_appear_in_same_group_by_clause!(users::name, posts::title);
/// allow_columns_to_appear_in_same_group_by_clause!(users::name, posts::id);
/// allow_columns_to_appear_in_same_group_by_clause!(posts::title, posts::id);
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! allow_columns_to_appear_in_same_group_by_clause {
    ($($left_path:tt)::+, $($right_path:tt)::+ $(,)?) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$($left_path)::+],
            $($right_path)::+,
        }
    };
    ($($left_path:tt)::+, $($right_path:tt)::+, $($other: tt)*) => {
        $crate::__diesel_impl_allow_in_same_group_by_clause! {
            left = [$($left_path)::+],
            $($right_path)::+,
            $($other)*
        }
        $crate::allow_columns_to_appear_in_same_group_by_clause! {
            $($right_path)::+,
            $($other)*
        }
    };
    ($last_col:ty,) => {};
    () => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_with_dollar_sign {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
    }
}

// The order of these modules is important (at least for those which have tests).
// Utility macros which don't call any others need to come first.
#[macro_use]
mod internal;
#[macro_use]
mod static_cond;
#[macro_use]
mod ops;

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    table! {
        foo.bars {
            id -> Integer,
            baz -> Text,
        }
    }

    mod my_types {
        #[derive(Debug, Clone, Copy, crate::sql_types::SqlType)]
        pub struct MyCustomType;
    }

    table! {
        use crate::sql_types::*;
        use crate::macros::tests::my_types::*;

        table_with_custom_types {
            id -> Integer,
            my_type -> MyCustomType,
        }
    }

    table! {
        use crate::sql_types::*;
        use crate::macros::tests::my_types::*;

        /// Table documentation
        ///
        /// some in detail documentation
        table_with_custom_type_and_id (a) {
            /// Column documentation
            ///
            /// some more details
            a -> Integer,
            my_type -> MyCustomType,
        }
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn table_with_custom_schema() {
        use crate::pg::Pg;
        let expected_sql = r#"SELECT "foo"."bars"."baz" FROM "foo"."bars" -- binds: []"#;
        assert_eq!(
            expected_sql,
            &crate::debug_query::<Pg, _>(&bars::table.select(bars::baz)).to_string()
        );
    }

    table! {
        use crate::sql_types;
        use crate::sql_types::*;

        table_with_arbitrarily_complex_types {
            id -> sql_types::Integer,
            qualified_nullable -> sql_types::Nullable<sql_types::Integer>,
            deeply_nested_type -> Nullable<Nullable<Integer>>,
            // This actually should work, but there appears to be a rustc bug
            // on the `AsExpression` bound for `EqAll` when the ty param is a projection
            // projected_type -> <Nullable<Integer> as sql_types::IntoNullable>::Nullable,
            //random_tuple -> (Integer, Integer),
        }
    }

    table!(
        foo {
            /// Column doc
            id -> Integer,

            #[sql_name = "type"]
            /// Also important to document this column
            mytype -> Integer,

            /// And this one
            #[sql_name = "bleh"]
            hey -> Integer,
        }
    );

    #[test]
    #[cfg(feature = "postgres")]
    fn table_with_column_renaming_postgres() {
        use crate::pg::Pg;
        let expected_sql = r#"SELECT "foo"."id", "foo"."type", "foo"."bleh" FROM "foo" WHERE ("foo"."type" = $1) -- binds: [1]"#;
        assert_eq!(
            expected_sql,
            crate::debug_query::<Pg, _>(&foo::table.filter(foo::mytype.eq(1))).to_string()
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn table_with_column_renaming_mysql() {
        use crate::mysql::Mysql;
        let expected_sql = r#"SELECT `foo`.`id`, `foo`.`type`, `foo`.`bleh` FROM `foo` WHERE (`foo`.`type` = ?) -- binds: [1]"#;
        assert_eq!(
            expected_sql,
            crate::debug_query::<Mysql, _>(&foo::table.filter(foo::mytype.eq(1))).to_string()
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn table_with_column_renaming_sqlite() {
        use crate::sqlite::Sqlite;
        let expected_sql = r#"SELECT `foo`.`id`, `foo`.`type`, `foo`.`bleh` FROM `foo` WHERE (`foo`.`type` = ?) -- binds: [1]"#;
        assert_eq!(
            expected_sql,
            crate::debug_query::<Sqlite, _>(&foo::table.filter(foo::mytype.eq(1))).to_string()
        );
    }

    table!(
        use crate::sql_types::*;

        /// Some documentation
        #[sql_name="mod"]
        /// Some more documentation
        bar {
            id -> Integer,
        }
    );

    #[test]
    #[cfg(feature = "postgres")]
    fn table_renaming_postgres() {
        use crate::pg::Pg;
        let expected_sql = r#"SELECT "mod"."id" FROM "mod" -- binds: []"#;
        assert_eq!(
            expected_sql,
            crate::debug_query::<Pg, _>(&bar::table.select(bar::id)).to_string()
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn table_renaming_mysql() {
        use crate::mysql::Mysql;
        let expected_sql = r#"SELECT `mod`.`id` FROM `mod` -- binds: []"#;
        assert_eq!(
            expected_sql,
            crate::debug_query::<Mysql, _>(&bar::table.select(bar::id)).to_string()
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn table_renaming_sqlite() {
        use crate::sqlite::Sqlite;
        let expected_sql = r#"SELECT `mod`.`id` FROM `mod` -- binds: []"#;
        assert_eq!(
            expected_sql,
            crate::debug_query::<Sqlite, _>(&bar::table.select(bar::id)).to_string()
        );
    }

    mod tests_for_allow_combined_group_by_syntax {
        use crate::table;

        table! {
            a(b) {
                b -> Text,
                c -> Text,
                d -> Text,
                e -> Text,
            }
        }

        table! {
            b(a) {
                a -> Text,
                c -> Text,
                d -> Text,
            }
        }

        table! {
            c(a) {
                a -> Text,
                b -> Text,
                d -> Text,
            }
        }

        // allow using table::column
        allow_columns_to_appear_in_same_group_by_clause!(a::b, b::a, a::d,);

        // allow using full paths
        allow_columns_to_appear_in_same_group_by_clause!(self::a::c, self::b::c, self::b::d,);

        use self::a::d as a_d;
        use self::b::d as b_d;
        use self::c::d as c_d;

        // allow using plain identifiers
        allow_columns_to_appear_in_same_group_by_clause!(a_d, b_d, c_d);

        // allow mixing all variants
        allow_columns_to_appear_in_same_group_by_clause!(c_d, self::b::a, a::e,);
    }
}
