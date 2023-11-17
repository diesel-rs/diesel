/// Declare a new alias for a table
///
/// This macro creates a value of the type [`Alias`](super::Alias)
///
/// Example usage
/// -------------
/// ```rust
/// # include!("../../doctest_setup.rs");
/// fn main() {
///     use schema::users;
///     let connection = &mut establish_connection();
///     let (users1, users2) = diesel::alias!(schema::users as user1, schema::users as user2);
///     let res = users1
///         .inner_join(users2.on(users1.field(users::id).eq(users2.field(users::id))))
///         .select((users1.fields((users::id, users::name)), users2.field(users::name)))
///         .order_by(users2.field(users::id))
///         .load::<((i32, String), String)>(connection);
///     assert_eq!(
///         res,
///         Ok(vec![
///             ((1, "Sean".to_owned()), "Sean".to_owned()),
///             ((2, "Tess".to_owned()), "Tess".to_owned()),
///         ]),
///     );
/// }
/// ```
///
///
/// Make type expressible
/// ---------------------
/// It may sometimes be useful to declare an alias at the module level, in such a way that the type
/// of a query using it can be expressed (to not declare it anonymously).
///
/// This can be achieved in the following way
/// ```rust
/// # include!("../../doctest_setup.rs");
/// use diesel::{query_source::Alias, dsl};
///
/// diesel::alias!(schema::users as users_alias: UsersAlias);
/// // or
/// diesel::alias!{
///     pub const USERS_ALIAS_2: Alias<UsersAlias2> = schema::users as users_alias_2;
/// }
///
/// fn some_function_that_returns_a_query_fragment(
/// ) -> dsl::InnerJoin<schema::posts::table, Alias<UsersAlias>>
/// {
///     schema::posts::table.inner_join(users_alias)
/// }
/// # fn main() {
/// #     some_function_that_returns_a_query_fragment();
/// #     schema::posts::table.inner_join(USERS_ALIAS_2);
/// # }
/// ```
///
/// Note that you may also use this form within a function, in the following way:
/// ```rust
/// # include!("../../doctest_setup.rs");
/// fn main() {
///     diesel::alias!(schema::users as users_alias: UsersAlias);
///     users_alias.inner_join(schema::posts::table);
/// }
/// ```
///
/// Troubleshooting and limitations
/// -------------------------------
/// If you encounter a **compilation error** where "the trait
/// `AppearsInFromClause<Alias<your_alias>>` is not implemented", when trying to use two aliases to
/// the same table within a single query note the following two limitations:
///  - You will need to declare these in a single `alias!` call.
///  - The path to the table module will have to be expressed in the exact same
///    manner. (That is, you can do `alias!(schema::users as user1, schema::users as user2)`
///    or `alias!(users as user1, users as user2)`, but not
///    `alias!(schema::users as user1, users as user2)`)
#[macro_export]
macro_rules! alias {
    ($($($table: ident)::+ as $alias: ident),* $(,)?) => {{
        $crate::alias!(NoConst $($($table)::+ as $alias: $alias,)*);
        ($($crate::query_source::Alias::<$alias>::default()),*)
    }};
    ($($($table: ident)::+ as $alias_name: ident: $alias_ty: ident),* $(,)?) => {
        $crate::alias! {
            $(
                pub const $alias_name: Alias<$alias_ty> = $($table)::+ as $alias_name;
            )*
        }
    };
    ($($vis: vis const $const_name: ident: Alias<$alias_ty: ident> = $($table: ident)::+ as $alias_sql_name: ident);* $(;)?) => {
        $crate::alias!(NoConst $($($table)::+ as $alias_sql_name: $vis $alias_ty,)*);
        $(
            #[allow(non_upper_case_globals)]
            $vis const $const_name: $crate::query_source::Alias::<$alias_ty> =
                $crate::query_source::Alias::new($alias_ty { table: $($table)::+::table });

            #[allow(non_camel_case_types)]
            $vis type $const_name = $crate::query_source::Alias::<$alias_ty>;
        )*
    };
    (NoConst $($($table: ident)::+ as $alias_sql_name: ident: $vis: vis $alias_ty: ident),* $(,)?) => {
        $(
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy)]
            $vis struct $alias_ty {
                table: $($table)::+::table,
            }

            impl $crate::query_source::AliasSource for $alias_ty {
                const NAME: &'static str = stringify!($alias_sql_name);
                type Target = $($table)::+::table;
                fn target(&self) -> &Self::Target { &self.table }
            }

            impl ::std::default::Default for $alias_ty {
                fn default() -> Self {
                    Self { table: $($table)::+::table }
                }
            }

            // impl AppearsInFromClause<Alias<$alias>> for Alias<$alias>
            impl $crate::internal::alias_macro::AliasAliasAppearsInFromClauseSameTable<$alias_ty, $($table)::+::table> for $alias_ty {
                type Count = $crate::query_source::Once;
            }
        )*
        $crate::__internal_alias_helper!($(table_ty = $($table)::+::table, table_tt = ($($table)::+), alias_ty = $alias_ty, alias_sql_name = $alias_sql_name;)*);
    };
}

#[macro_export]
#[doc(hidden)]
/// This only exists to hide internals from the doc
// The `($left_sql_name) != ($right_sql_name)` condition is not perfect because a user could still
// cause runtime errors by declaring two aliases to different tables with the same name then use
// them in the same query, but that would almost have to be voluntary, so this alone should prevent
// most mistakes.
macro_rules! __internal_alias_helper {
    (
        table_ty = $left_table_ty: ty, table_tt = $left_table_tt: tt, alias_ty = $left_alias: ident, alias_sql_name = $left_sql_name: ident;
        $(table_ty = $right_table_ty: ty, table_tt = $right_table_tt: tt, alias_ty = $right_alias: ident, alias_sql_name = $right_sql_name: ident;)+
    ) => {
        $(
            $crate::static_cond!{if ($left_table_tt) == ($right_table_tt) {
                $crate::static_cond!{if ($left_sql_name) != ($right_sql_name) {
                    impl $crate::internal::alias_macro::AliasAliasAppearsInFromClauseSameTable<$left_alias, $left_table_ty>
                        for $right_alias
                    {
                        type Count = $crate::query_source::Never;
                    }
                    impl $crate::internal::alias_macro::AliasAliasAppearsInFromClauseSameTable<$right_alias, $left_table_ty>
                        for $left_alias
                    {
                        type Count = $crate::query_source::Never;
                    }
                }}
            }}
        )*
        $crate::__internal_alias_helper!($(table_ty = $right_table_ty, table_tt = $right_table_tt, alias_ty = $right_alias, alias_sql_name = $right_sql_name;)+);
    };

    (table_ty = $left_table_ty: ty, table_tt = $left_table_tt: tt, alias_ty = $left_alias: ident, alias_sql_name = $left_sql_name: ident;) => {};
    () => {};
}
