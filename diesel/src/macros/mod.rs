#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_column {
    ($($table:ident)::*, $column_name:ident -> $Type:ty) => {
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy)]
        pub struct $column_name;

        impl $crate::expression::Expression for $column_name {
            type SqlType = $Type;
        }

        impl<DB> $crate::query_builder::QueryFragment<DB> for $column_name where
            DB: $crate::backend::Backend,
            <$($table)::* as QuerySource>::FromClause: QueryFragment<DB>,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!($($table)::*.from_clause().to_sql(out));
                out.push_sql(".");
                out.push_identifier(stringify!($column_name))
            }

            fn collect_binds(&self, _out: &mut DB::BindCollector) -> $crate::result::QueryResult<()> {
                Ok(())
            }

            fn is_safe_to_cache_prepared(&self) -> bool {
                true
            }
        }

        impl_query_id!($column_name);

        impl SelectableExpression<$($table)::*> for $column_name {
        }

        impl AppearsOnTable<$($table)::*> for $column_name {
        }

        impl<Right, Kind> SelectableExpression<
            Join<$($table)::*, Right, Kind>,
        > for $column_name where
            $column_name: AppearsOnTable<Join<$($table)::*, Right, Kind>>,
        {
        }

        impl<Left> SelectableExpression<
            Join<Left, $($table)::*, Inner>,
        > for $column_name where
            Left: $crate::JoinTo<$($table)::*, Inner>
        {
        }

        impl<Right> AppearsOnTable<
            Join<$($table)::*, Right, Inner>,
        > for $column_name where
            Right: Table,
            $($table)::*: $crate::JoinTo<Right, Inner>
        {
        }

        impl<Left> AppearsOnTable<
            Join<Left, $($table)::*, Inner>,
        > for $column_name where
            Left: $crate::JoinTo<$($table)::*, Inner>
        {
        }

        impl<Right> AppearsOnTable<
            Join<$($table)::*, Right, LeftOuter>,
        > for $column_name where
            Right: Table,
            $($table)::*: $crate::JoinTo<Right, LeftOuter>
        {
        }

        impl<Left> AppearsOnTable<
            Join<Left, $($table)::*, LeftOuter>,
        > for $column_name where
            Left: $crate::JoinTo<$($table)::*, LeftOuter>
        {
        }

        impl $crate::expression::NonAggregate for $column_name {}

        impl $crate::query_source::Column for $column_name {
            type Table = $($table)::*;

            fn name() -> &'static str {
                stringify!($column_name)
            }
        }

        impl<T> $crate::EqAll<T> for $column_name where
            T: $crate::expression::AsExpression<$Type>,
            $crate::expression::helper_types::Eq<$column_name, T>: $crate::Expression<SqlType=$crate::types::Bool>,
        {
            type Output = $crate::expression::helper_types::Eq<Self, T>;

            fn eq_all(self, rhs: T) -> Self::Output {
                $crate::ExpressionMethods::eq(self, rhs)
            }
        }
    }
}

/// Specifies that a table exists, and what columns it has. This will create a
/// new public module, with the same name, as the name of the table. In this
/// module, you'll find a unit struct named `table`, and a unit struct with the
/// names of each of the columns. In the definition, you can also specify an
/// additional set of columns which exist, but should not be selected by default
/// (for example, for things like full text search)
///
/// By default this allows a maximum of 16 columns per table, in order to reduce
/// compilation time. You can increase this limit to 26 by enabling the
/// `large-tables` feature, or up to 52 by enabling the `huge-tables` feature.
/// Enabling `huge-tables` will *substantially* increase compile times.
///
/// Example usage
/// -------------
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// table! {
///     users {
///         id -> Integer,
///         name -> VarChar,
///         favorite_color -> Nullable<VarChar>,
///     }
/// }
/// # fn main() {}
/// ```
///
/// You may also specify a primary key if it's called something other than `id`.
/// Tables with no primary key, or composite primary containing more than 3
/// columns are not supported.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// table! {
///     users (non_standard_primary_key) {
///         non_standard_primary_key -> Integer,
///         name -> VarChar,
///         favorite_color -> Nullable<VarChar>,
///     }
/// }
/// # fn main() {}
/// ```
///
/// For tables with composite primary keys, list all of the columns in the
/// primary key.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// table! {
///     followings (user_id, post_id) {
///         user_id -> Integer,
///         post_id -> Integer,
///         favorited -> Bool,
///     }
/// }
/// # fn main() {
/// #     use diesel::prelude::*;
/// #     use self::followings::dsl::*;
/// #     // Poor man's assert_eq! -- since this is type level this would fail
/// #     // to compile if the wrong primary key were generated
/// #     let (user_id {}, post_id {}) = followings.primary_key();
/// # }
/// ```
///
/// If you are using types that aren't from Diesel's core types, you can specify
/// which types to import. Note that the path given has to be an absolute path
/// relative to the crate root. You cannot use `self` or `super`.
///
/// ```
/// #[macro_use] extern crate diesel;
/// # /*
/// extern crate diesel_full_text_search;
/// # */
/// # mod diesel_full_text_search {
/// #     pub struct TsVector;
/// # }
///
/// table! {
///     use diesel::types::*;
///     use diesel_full_text_search::*;
///
///     posts {
///         id -> Integer,
///         title -> Text,
///         keywords -> TsVector,
///     }
/// }
/// # fn main() {}
/// ```
///
/// This module will also contain several helper types:
///
/// dsl
/// ---
///
/// This simply re-exports the table, renamed to the same name as the module,
/// and each of the columns. This is useful to glob import when you're dealing
/// primarily with one table, to allow writing `users.filter(name.eq("Sean"))`
/// instead of `users::table.filter(users::name.eq("Sean"))`.
///
/// all_columns
/// -----------
///
/// A constant will be assigned called `all_columns`. This is what will be
/// selected if you don't otherwise specify a select clause. It's type will be
/// `table::AllColumns`. You can also get this value from the
/// `Table::all_columns` function.
///
/// star
/// ----
///
/// This will be the qualified "star" expression for this table (e.g.
/// `users.*`). Internally, we read columns by index, not by name, so this
/// column is not safe to read data out of, and it has had it's SQL type set to
/// `()` to prevent accidentally using it as such. It is sometimes useful for
/// count statements however. It can also be accessed through the `Table.star()`
/// method.
///
/// SqlType
/// -------
///
/// A type alias called `SqlType` will be created. It will be the SQL type of
/// `all_columns`. The SQL type is needed for things like [returning boxed
/// queries][boxed_queries].
///
/// [boxed_queries]: prelude/trait.BoxedDsl.html#example-1
///
/// BoxedQuery
/// ----------
///
/// ```ignore
/// pub type BoxedQuery<'a, DB, ST = SqlType> = BoxedSelectStatement<'a, ST, table, DB>;
/// ```
#[macro_export]
macro_rules! table {
    // Put `use` statements at the end because macro_rules! cannot figure out
    // if `use` is an ident or not (hint: It's not)
    (
        use $($import:tt)::+; $($rest:tt)+
    ) => {
        table!($($rest)+ use $($import)::+;);
    };

    // Add the primary key if it's not present
    (
        $($table_name:ident).+ {$($body:tt)*}
        $($imports:tt)*
    ) => {
        table! {
            $($table_name).+ (id) {$($body)*} $($imports)*
        }
    };

    // Add the schema name if it's not present
    (
        $name:ident $(($($pk:ident),+))* {$($body:tt)*}
        $($imports:tt)*
    ) => {
        table! {
            public . $name $(($($pk),+))* {$($body)*} $($imports)*
        }
    };

    // Import `diesel::types::*` if no imports were given
    (
        $($table_name:ident).+ $(($($pk:ident),+))* {$($body:tt)*}
    ) => {
        table! {
            $($table_name).+ $(($($pk),+))* {$($body)*}
            use $crate::types::*;
        }
    };

    // Terminal with single-column pk
    (
        $schema_name:ident . $name:ident ($pk:ident) $body:tt
        $($imports:tt)+
    ) => {
        table_body! {
            $schema_name . $name ($pk) $body $($imports)+
        }
    };

    // Terminal with composite pk (add a trailing comma)
    (
        $schema_name:ident . $name:ident ($pk:ident, $($composite_pk:ident),+) $body:tt
        $($imports:tt)+
    ) => {
        table_body! {
            $schema_name . $name ($pk, $($composite_pk,)+) $body $($imports)+
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! table_body {
    (
        $schema_name:ident . $name:ident ($pk:ident) {
            $($column_name:ident -> $Type:ty,)+
        }
        $(use $($import:tt)::+;)+
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = columns::$pk,
            primary_key_expr = columns::$pk,
            columns = [$($column_name -> $Type,)+],
            imports = ($($($import)::+),+),
        }
    };

    (
        $schema_name:ident . $name:ident ($($pk:ident,)+) {
            $($column_name:ident -> $Type:ty,)+
        }
        $(use $($import:tt)::+;)+
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = ($(columns::$pk,)+),
            primary_key_expr = ($(columns::$pk,)+),
            columns = [$($column_name -> $Type,)+],
            imports = ($($($import)::+),+),
        }
    };

    (
        schema_name = $schema_name:ident,
        table_name = $table_name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $column_ty:ty,)+],
        imports = ($($($import:tt)::+),+),
    ) => {
        pub mod $table_name {
            #![allow(dead_code)]
            use $crate::{
                QuerySource,
                Table,
            };
            use $crate::associations::HasTable;
            use $crate::query_builder::*;
            use $crate::query_builder::nodes::Identifier;
            $(use $($import)::+;)+
            pub use self::columns::*;

            /// Re-exports all of the columns of this table, as well as the
            /// table struct renamed to the module name. This is meant to be
            /// glob imported for functions which only deal with one table.
            pub mod dsl {
                pub use super::columns::{$($column_name),+};
                pub use super::table as $table_name;
            }

            #[allow(non_upper_case_globals, dead_code)]
            /// A tuple of all of the columns on this table
            pub const all_columns: ($($column_name,)+) = ($($column_name,)+);

            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy)]
            /// The actual table struct
            ///
            /// This is the type which provides the base methods of the query
            /// builder, such as `.select` and `.filter`.
            pub struct table;

            impl table {
                #[allow(dead_code)]
                /// Represents `table_name.*`, which is sometimes necessary
                /// for efficient count queries. It cannot be used in place of
                /// `all_columns`
                pub fn star(&self) -> star {
                    star
                }
            }

            /// The SQL type of all of the columns on this table
            pub type SqlType = ($($column_ty,)+);

            /// Helper type for reperesenting a boxed query from this table
            pub type BoxedQuery<'a, DB, ST = SqlType> = BoxedSelectStatement<'a, ST, table, DB>;

            __diesel_table_query_source_impl!(table, $schema_name, $table_name);

            impl AsQuery for table {
                type SqlType = SqlType;
                type Query = SelectStatement<Self>;

                fn as_query(self) -> Self::Query {
                    SelectStatement::simple(self)
                }
            }

            impl Table for table {
                type PrimaryKey = $primary_key_ty;
                type AllColumns = ($($column_name,)+);

                fn primary_key(&self) -> Self::PrimaryKey {
                    $primary_key_expr
                }

                fn all_columns() -> Self::AllColumns {
                    ($($column_name,)+)
                }
            }

            impl HasTable for table {
                type Table = Self;

                fn table() -> Self::Table {
                    table
                }
            }

            impl IntoUpdateTarget for table {
                type WhereClause = <<Self as AsQuery>::Query as IntoUpdateTarget>::WhereClause;

                fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
                    self.as_query().into_update_target()
                }
            }

            impl_query_id!(table);

            /// Contains all of the columns of this table
            pub mod columns {
                use super::table;
                use $crate::{Table, Expression, SelectableExpression, AppearsOnTable, QuerySource};
                use $crate::backend::Backend;
                use $crate::query_builder::{QueryBuilder, BuildQueryResult, QueryFragment};
                use $crate::query_source::joins::{Join, Inner, LeftOuter};
                use $crate::result::QueryResult;
                $(use $($import)::+;)+

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy)]
                /// Represents `table_name.*`, which is sometimes needed for
                /// efficient count queries. It cannot be used in place of
                /// `all_columns`, and has a `SqlType` of `()` to prevent it
                /// being used that way
                pub struct star;

                impl Expression for star {
                    type SqlType = ();
                }

                impl<DB: Backend> QueryFragment<DB> for star where
                    <table as QuerySource>::FromClause: QueryFragment<DB>,
                {
                    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
                        try!(table.from_clause().to_sql(out));
                        out.push_sql(".*");
                        Ok(())
                    }

                    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
                        Ok(())
                    }

                    fn is_safe_to_cache_prepared(&self) -> bool {
                        true
                    }
                }

                impl SelectableExpression<table> for star {
                }

                impl AppearsOnTable<table> for star {
                }

                $(__diesel_column!(table, $column_name -> $column_ty);)+
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_table_query_source_impl {
    ($table_struct:ident, public, $table_name:ident) => {
        impl QuerySource for $table_struct {
            type FromClause = Identifier<'static>;
            type DefaultSelection = <Self as Table>::AllColumns;

            fn from_clause(&self) -> Self::FromClause {
                Identifier(stringify!($table_name))
            }

            fn default_selection(&self) -> Self::DefaultSelection {
                Self::all_columns()
            }
        }
    };

    ($table_struct:ident, $schema_name:ident, $table_name:ident) => {
        impl QuerySource for $table_struct {
            type FromClause = $crate::query_builder::nodes::
                InfixNode<'static, Identifier<'static>, Identifier<'static>>;
            type DefaultSelection = <Self as Table>::AllColumns;

            fn from_clause(&self) -> Self::FromClause {
                $crate::query_builder::nodes::InfixNode::new(
                    Identifier(stringify!($schema_name)),
                    Identifier(stringify!($table_name)),
                    ".",
                )
            }

            fn default_selection(&self) -> Self::DefaultSelection {
                Self::all_columns()
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! joinable {
    ($child:ident -> $parent:ident ($source:ident)) => {
        joinable_inner!($child::table => $parent::table : ($child::$source = $parent::table));
        joinable_inner!($parent::table => $child::table : ($child::$source = $parent::table));
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! joinable_inner {
    ($left_table:path => $right_table:path : ($foreign_key:path = $parent_table:path)) => {
        joinable_inner!(
            left_table_ty = $left_table,
            right_table_ty = $right_table,
            right_table_expr = $right_table,
            foreign_key = $foreign_key,
            primary_key_ty = <$parent_table as $crate::query_source::Table>::PrimaryKey,
            primary_key_expr = $parent_table.primary_key(),
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
        impl<JoinType> $crate::JoinTo<$right_table_ty, JoinType> for $left_table_ty {
            type JoinClause = $crate::query_builder::nodes::Join<
                <$left_table_ty as $crate::QuerySource>::FromClause,
                <$right_table_ty as $crate::QuerySource>::FromClause,
                $crate::expression::helper_types::Eq<
                    $crate::expression::nullable::Nullable<$foreign_key>,
                    $crate::expression::nullable::Nullable<$primary_key_ty>,
                >,
                JoinType,
            >;

            fn join_clause(&self, join_type: JoinType) -> Self::JoinClause {
                use $crate::{QuerySource, ExpressionMethods};

                $crate::query_builder::nodes::Join::new(
                    self.from_clause(),
                    $right_table_expr.from_clause(),
                    $foreign_key.nullable().eq($primary_key_expr.nullable()),
                    join_type,
                )
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! join_through {
    ($parent:ident -> $through:ident -> $child:ident) => {
        impl<JoinType: Copy> $crate::JoinTo<$child::table, JoinType> for $parent::table {
            type JoinClause = <
                <$parent::table as $crate::JoinTo<$through::table, JoinType>>::JoinClause
                as $crate::query_builder::nodes::CombinedJoin<
                    <$through::table as $crate::JoinTo<$child::table, JoinType>>::JoinClause,
                >>::Output;

            fn join_clause(&self, join_type: JoinType) -> Self::JoinClause {
                use $crate::query_builder::nodes::CombinedJoin;
                let parent_to_through = $crate::JoinTo::<$through::table, JoinType>
                    ::join_clause(&$parent::table, join_type);
                let through_to_child = $crate::JoinTo::<$child::table, JoinType>
                    ::join_clause(&$through::table, join_type);
                parent_to_through.combine_with(through_to_child)
            }
        }
    }
}

/// Takes a query QueryFragment expression as an argument and returns a string
/// of SQL with placeholders for the dynamic values.
///
/// # Example
///
/// ### Returning SQL from a count statment:
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # table! {
/// #     users {
/// #         id -> Timestamp,
/// #         n -> Integer,
/// #     }
/// # }
/// # // example requires setup for users table
/// # use self::users::dsl::*;
/// # use diesel::*;
/// #
/// # fn main() {
/// let sql = debug_sql!(users.count());
/// assert_eq!(sql, "SELECT COUNT(*) FROM `users`");
/// # }
/// ```
#[macro_export]
macro_rules! debug_sql {
    ($query:expr) => {{
        use $crate::query_builder::{QueryFragment, QueryBuilder};
        use $crate::query_builder::debug::DebugQueryBuilder;
        let mut query_builder = DebugQueryBuilder::new();
        QueryFragment::<$crate::backend::Debug>::to_sql(&$query, &mut query_builder).unwrap();
        query_builder.finish()
    }};
}

/// Takes takes a query QueryFragment expression as an argument and prints out
/// the SQL with placeholders for the dynamic values.
///
/// # Example
///
/// ### Printing SQL from a count statment:
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # table! {
/// #     users {
/// #         id -> Timestamp,
/// #         n -> Integer,
/// #     }
/// # }
/// # // example requires setup for users table
/// # use self::users::dsl::*;
/// # use diesel::*;
/// #
/// # fn main() {
/// print_sql!(users.count());
/// # }
/// ```
#[macro_export]
macro_rules! print_sql {
    ($query:expr) => {
        println!("{}", &debug_sql!($query));
    };
}

// The order of these modules is important (at least for those which have tests).
// Utililty macros which don't call any others need to come first.
#[macro_use] mod internal;
#[macro_use] mod parse;
#[macro_use] mod query_id;
#[macro_use] mod static_cond;
#[macro_use] mod macros_from_codegen;

#[macro_use] mod as_changeset;
#[macro_use] mod associations;
#[macro_use] mod identifiable;
#[macro_use] mod insertable;

#[cfg(test)]
mod tests {
    use prelude::*;

    table! {
        foo.bars {
            id -> Integer,
            baz -> Text,
        }
    }

    mod my_types {
        pub struct MyCustomType;
    }

    table! {
        use types::*;
        use macros::tests::my_types::*;

        table_with_custom_types {
            id -> Integer,
            my_type -> MyCustomType,
        }
    }

    #[test]
    fn table_with_custom_schema() {
        let expected_sql = "SELECT `foo`.`bars`.`baz` FROM `foo`.`bars`";
        assert_eq!(expected_sql, debug_sql!(bars::table.select(bars::baz)));
    }
}
