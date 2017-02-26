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
            type SqlTypeForSelect = $Type;
        }

        impl AppearsOnTable<$($table)::*> for $column_name {
        }

        impl<Source, Predicate> SelectableExpression<
            $crate::query_source::filter::FilteredQuerySource<Source, Predicate>,
        > for $column_name where
            $column_name: SelectableExpression<Source>,
        {
            type SqlTypeForSelect = $Type;
        }

        impl<Source, Predicate> AppearsOnTable<
            $crate::query_source::filter::FilteredQuerySource<Source, Predicate>,
        > for $column_name where
            $column_name: AppearsOnTable<Source>,
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
    (
        $name:ident $body:tt
    ) => {
        table! {
            public . $name (id) $body
        }
    };

    (
        $schema_name:ident . $name:ident $body:tt
    ) => {
        table! {
            $schema_name . $name (id) $body
        }
    };

    (
        $name:ident $pk:tt $body:tt
    ) => {
        table! {
            public . $name $pk $body
        }
    };

    (
        $schema_name:ident . $name:ident ($pk:ident) $body:tt
    ) => {
        table_body! {
            $schema_name . $name ($pk) $body
        }
    };

    (
        $schema_name:ident . $name:ident ($pk:ident, $($composite_pk:ident),+) {
            $($column_name:ident -> $Type:ty,)+
        }
    ) => {
        table_body! {
            $schema_name . $name ($pk, $($composite_pk,)+) {
                $($column_name -> $Type,)+
            }
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
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = columns::$pk,
            primary_key_expr = columns::$pk,
            columns = [$($column_name -> $Type,)+],
        }
    };

    (
        $schema_name:ident . $name:ident ($($pk:ident,)+) {
            $($column_name:ident -> $Type:ty,)+
        }
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = ($(columns::$pk,)+),
            primary_key_expr = ($(columns::$pk,)+),
            columns = [$($column_name -> $Type,)+],
        }
    };

    (
        schema_name = $schema_name:ident,
        table_name = $table_name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $column_ty:ty,)+],
    ) => {
        pub mod $table_name {
            #![allow(dead_code)]
            #![allow(unused_imports)] // FIXME: Once we revamp type imports this can be removed
            use $crate::{
                QuerySource,
                Table,
            };
            use $crate::associations::HasTable;
            use $crate::query_builder::*;
            use $crate::query_builder::nodes::{Identifier, InfixNode};
            use $crate::types::*;
            pub use self::columns::*;

            pub mod dsl {
                pub use super::columns::{$($column_name),+};
                pub use super::table as $table_name;
            }

            #[allow(non_upper_case_globals, dead_code)]
            pub const all_columns: ($($column_name,)+) = ($($column_name,)+);

            #[allow(non_camel_case_types, missing_debug_implementations)]
            #[derive(Debug, Clone, Copy)]
            pub struct table;

            impl table {
                #[allow(dead_code)]
                pub fn star(&self) -> star {
                    star
                }
            }

            pub type SqlType = ($($column_ty,)+);

            pub type BoxedQuery<'a, DB, ST = SqlType> = BoxedSelectStatement<'a, ST, table, DB>;

            __diesel_table_query_source_impl!(table, $schema_name, $table_name);

            impl AsQuery for table {
                type SqlType = SqlType;
                type Query = SelectStatement<($($column_name,)+), Self>;

                fn as_query(self) -> Self::Query {
                    SelectStatement::simple(all_columns, self)
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
                type WhereClause = ();

                fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
                    UpdateTarget {
                        table: self,
                        where_clause: None,
                    }
                }
            }

            impl_query_id!(table);

            pub mod columns {
                use super::table;
                use $crate::{Table, Expression, SelectableExpression, AppearsOnTable, QuerySource};
                use $crate::backend::Backend;
                use $crate::query_builder::{QueryBuilder, BuildQueryResult, QueryFragment};
                use $crate::result::QueryResult;
                use $crate::types::*;

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy)]
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
                    type SqlTypeForSelect = Self::SqlType;
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

            fn from_clause(&self) -> Self::FromClause {
                Identifier(stringify!($table_name))
            }
        }
    };

    ($table_struct:ident, $schema_name:ident, $table_name:ident) => {
        impl QuerySource for $table_struct {
            type FromClause = InfixNode<'static, Identifier<'static>, Identifier<'static>>;

            fn from_clause(&self) -> Self::FromClause {
                InfixNode::new(
                    Identifier(stringify!($schema_name)),
                    Identifier(stringify!($table_name)),
                    ".",
                )
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
macro_rules! select_column_workaround {
    ($parent:ident -> $child:ident ($($column_name:ident),+)) => {
        $(select_column_inner!($parent::table, $child::table, $parent::$column_name);)+
        select_column_inner!($parent::table, $child::table, $parent::star);
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! select_column_inner {
    ($parent:ty, $child:ty, $column:ty $(,)*) => {
        impl $crate::expression::SelectableExpression<
            $crate::query_source::InnerJoinSource<$child, $parent>,
        > for $column {
            type SqlTypeForSelect = <Self as $crate::Expression>::SqlType;
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::InnerJoinSource<$parent, $child>,
        > for $column {
            type SqlTypeForSelect = <Self as $crate::Expression>::SqlType;
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::LeftOuterJoinSource<$child, $parent>,
        > for $column {
            type SqlTypeForSelect = <<Self as $crate::Expression>::SqlType
                as $crate::types::IntoNullable>::Nullable;
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::LeftOuterJoinSource<$parent, $child>,
        > for $column {
            type SqlTypeForSelect = <Self as $crate::Expression>::SqlType;
        }

        impl $crate::expression::AppearsOnTable<
            $crate::query_source::InnerJoinSource<$child, $parent>,
        > for $column {
        }

        impl $crate::expression::AppearsOnTable<
            $crate::query_source::InnerJoinSource<$parent, $child>,
        > for $column {
        }

        impl $crate::expression::AppearsOnTable<
            $crate::query_source::LeftOuterJoinSource<$child, $parent>,
        > for $column {
        }

        impl $crate::expression::AppearsOnTable<
            $crate::query_source::LeftOuterJoinSource<$parent, $child>,
        > for $column {
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

    #[test]
    fn table_with_custom_schema() {
        let expected_sql = "SELECT `foo`.`bars`.`baz` FROM `foo`.`bars`";
        assert_eq!(expected_sql, debug_sql!(bars::table.select(bars::baz)));
    }
}
