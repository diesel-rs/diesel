// FIXME(https://github.com/rust-lang/rust/issues/19630) Remove this work-around
#[macro_export]
#[doc(hidden)]
macro_rules! diesel_internal_expr_conversion {
    ($e:expr) => { $e }
}

#[macro_export]
#[doc(hidden)]
macro_rules! column {
    ($($table:ident)::*, $column_name:ident -> $Type:ty) => {
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy)]
        pub struct $column_name;

        impl $crate::expression::Expression for $column_name {
            type SqlType = $Type;
        }

        impl $crate::query_builder::QueryFragment for $column_name {
            fn to_sql(&self, out: &mut $crate::query_builder::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!(out.push_identifier($($table)::*::name()));
                out.push_sql(".");
                out.push_identifier(stringify!($column_name))
            }
        }

        impl $crate::expression::SelectableExpression<$($table)::*> for $column_name {}

        impl<'a, ST, Left, Right> SelectableExpression<
            $crate::WithQuerySource<'a, Left, Right>, ST> for $column_name where
            ST: NativeSqlType,
            $column_name: SelectableExpression<Left, ST>
        {
        }

        impl $crate::expression::NonAggregate for $column_name {}

        impl $crate::query_source::Column for $column_name {
            type Table = $($table)::*;

            fn name() -> &'static str {
                stringify!($column_name)
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
/// Example usage
/// -------------
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// table! {
///     users {
///         id -> Serial,
///         name -> VarChar,
///         favorite_color -> Nullable<VarChar>,
///     }
/// }
/// # fn main() {}
/// ```
///
/// More complex usage:
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// table! {
///     users (non_standard_primary_key) {
///         non_standard_primary_key -> Serial,
///         name -> VarChar,
///         favorite_color -> Nullable<VarChar>,
///     } no select {
///         complex_index_column -> Text,
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
/// A constant will be assigned called `all_columns`, which will be a tuple of
/// all the columns that aren't in the "no select" group. This is what will be
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
#[macro_export]
macro_rules! table {
    (
        $name:ident {
            $($column_name:ident -> $Type:ty,)+
        }
    ) => {
        table! {
            $name (id) {
                $($column_name -> $Type,)+
            }
        }
    };
    (
        $name:ident ($pk:ident) {
            $($column_name:ident -> $Type:ty,)+
        }
    ) => {
        table! {
            $name ($pk) {
                $($column_name -> $Type,)+
            } no select {}
        }
    };
    (
        $name:ident {
            $($column_name:ident -> $Type:ty,)+
        } no select {
            $($no_select_column_name:ident -> $no_select_type:ty,)*
        }
    ) => {
        table! {
            $name (id) {
                $($column_name -> $Type,)+
            } no select {
                $($no_select_column_name -> $no_select_type,)*
            }
        }
    };
    (
        $name:ident ($pk:ident) {
            $($column_name:ident -> $Type:ty,)+
        } no select {
            $($no_select_column_name:ident -> $no_select_type:ty,)*
        }
    ) => {
        table_body! {
            $name ($pk) {
                $($column_name -> $Type,)+
            } no select {
                $($no_select_column_name -> $no_select_type,)*
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! table_body {
    (
        $name:ident ($pk:ident) {
            $($column_name:ident -> $Type:ty,)+
        } no select {
            $($no_select_column_name:ident -> $no_select_type:ty,)*
        }
    ) => {
        pub mod $name {
            use $crate::{
                QuerySource,
                Table,
            };
            use $crate::query_builder::*;
            use $crate::types::*;
            pub use self::columns::*;

            pub mod dsl {
                pub use super::columns::{$($column_name),+};
                pub use super::table as $name;
            }

            #[allow(non_upper_case_globals, dead_code)]
            pub const all_columns: ($($column_name),+) = ($($column_name),+);

            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            pub struct table;

            impl table {
                #[allow(dead_code)]
                pub fn star(&self) -> star {
                    star
                }
            }

            pub type SqlType = ($($Type),+);

            impl QuerySource for table {
                fn from_clause(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                    out.push_identifier(stringify!($name))
                }
            }

            impl AsQuery for table {
                type SqlType = SqlType;
                type Query = SelectStatement<SqlType, ($($column_name),+), Self>;

                fn as_query(self) -> Self::Query {
                    SelectStatement::simple(all_columns, self)
                }
            }

            impl Table for table {
                type PrimaryKey = columns::$pk;
                type AllColumns = ($($column_name),+);

                fn name() -> &'static str {
                    stringify!($name)
                }

                fn primary_key(&self) -> Self::PrimaryKey {
                    columns::$pk
                }

                fn all_columns() -> Self::AllColumns {
                    ($($column_name),+)
                }
            }

            pub mod columns {
                use super::table;
                use $crate::{Table, Column, Expression, SelectableExpression};
                use $crate::query_builder::{QueryBuilder, BuildQueryResult, QueryFragment};
                use $crate::types::*;

                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone, Copy)]
                pub struct star;

                impl Expression for star {
                    type SqlType = ();
                }

                impl QueryFragment for star {
                    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                        try!(out.push_identifier(table::name()));
                        out.push_sql(".*");
                        Ok(())
                    }
                }

                impl SelectableExpression<table> for star {}

                $(column!(table, $column_name -> $Type);)+
                $(column!(table, $no_select_column_name -> $no_select_type);)*
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! joinable {
    ($child:ident -> $parent:ident ($source:ident = $target:ident)) => {
        joinable_inner!($child -> $parent ($source = $target));
        joinable_inner!($parent -> $child ($target = $source));
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! joinable_inner {
    ($child:ident -> $parent:ident ($source:ident = $target:ident)) => {
        impl $crate::JoinTo<$parent::table> for $child::table {
            fn join_sql(&self, out: &mut $crate::query_builder::QueryBuilder)
                -> $crate::query_builder::BuildQueryResult
            {
                use $crate::query_builder::QueryFragment;
                try!($parent::table.from_clause(out));
                out.push_sql(" ON ");

                $child::$source.nullable().eq($parent::$target.nullable()).to_sql(out)
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! select_column_workaround {
    ($parent:ident -> $child:ident ($($column_name:ident),+)) => {
        $(select_column_inner!($parent -> $child $column_name);)+
        select_column_inner!($parent -> $child star);
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! select_column_inner {
    ($parent:ident -> $child:ident $column_name:ident) => {
        impl $crate::expression::SelectableExpression<
            $crate::query_source::InnerJoinSource<$child::table, $parent::table>,
        > for $parent::$column_name
        {
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::InnerJoinSource<$parent::table, $child::table>,
        > for $parent::$column_name
        {
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::LeftOuterJoinSource<$child::table, $parent::table>,
            <<$parent::$column_name as $crate::Expression>::SqlType
                as $crate::types::IntoNullable>::Nullable,
        > for $parent::$column_name
        {
        }

        impl $crate::expression::SelectableExpression<
            $crate::query_source::LeftOuterJoinSource<$parent::table, $child::table>,
        > for $parent::$column_name
        {
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! join_through {
    ($parent:ident -> $through:ident -> $child:ident) => {
        impl $crate::JoinTo<$child::table> for $parent::table {
            fn join_sql(&self, out: &mut $crate::query_builder::QueryBuilder)
                -> $crate::query_builder::BuildQueryResult
            {
                try!($crate::JoinTo::<$through::table>::join_sql(&$parent::table, out));
                out.push_sql(" INNER JOIN ");
                $crate::JoinTo::<$child::table>::join_sql(&$through::table, out)
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
/// #
/// # ```rust
/// # // example requires setup for users table
/// # use diesel::users::dsl::*;
/// # use diesel::query_builder::QueryFragment;
/// #
/// # fn main() {
/// let sql = debug_sql!(users.count());
/// assert_eq!(sql, "SELECT COUNT(*) FROM users");
/// # }
/// # ```
#[macro_export]
macro_rules! debug_sql {
    ($query:expr) => {{
        use diesel::query_builder::QueryFragment;
        let mut query_builder = DebugQueryBuilder::new();
        QueryFragment::to_sql(&$query, &mut query_builder).unwrap();
        query_builder.sql
    }};
}

/// Takes takes a query QueryFragment expression as an argument and prints out
/// the SQL with placeholders for the dynamic values.
///
/// # Example
///
/// ### Printing SQL from a count statment:
/// #
/// # ```rust
/// # // example requires setup for users table
/// # use diesel::users::dsl::*;
/// # use diesel::query_builder::QueryFragment;
/// #
/// # fn main() {
/// print_sql!(users.count());
/// # }
/// # ```
#[macro_export]
macro_rules! print_sql {
    ($query:expr) => {
        println!("{}", &debug_sql!($query));
    };
}
