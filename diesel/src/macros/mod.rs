#![cfg_attr(rustfmt, rustfmt_skip)] // https://github.com/rust-lang-nursery/rustfmt/issues/2755

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_column {
    (
        table = $table:ident,
        name = $column_name:ident,
        sql_name = $sql_name:expr,
        ty = ($($Type:tt)*),
        meta = [$($meta:tt)*],
    ) => {
        $($meta)*
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy, QueryId, Default)]
        pub struct $column_name;

        impl $crate::expression::Expression for $column_name {
            type SqlType = $($Type)*;
        }

        impl<DB> $crate::query_builder::QueryFragment<DB> for $column_name where
            DB: $crate::backend::Backend,
            <$table as QuerySource>::FromClause: QueryFragment<DB>,
        {
            fn walk_ast(&self, mut out: $crate::query_builder::AstPass<DB>) -> $crate::result::QueryResult<()> {
                $table.from_clause().walk_ast(out.reborrow())?;
                out.push_sql(".");
                out.push_identifier($sql_name)
            }
        }

        impl SelectableExpression<$table> for $column_name {
        }

        impl<QS> AppearsOnTable<QS> for $column_name where
            QS: AppearsInFromClause<$table, Count=Once>,
        {
        }

        impl<Left, Right> SelectableExpression<
            Join<Left, Right, LeftOuter>,
        > for $column_name where
            $column_name: AppearsOnTable<Join<Left, Right, LeftOuter>>,
            Left: AppearsInFromClause<$table, Count=Once>,
            Right: AppearsInFromClause<$table, Count=Never>,
        {
        }

        impl<Left, Right> SelectableExpression<
            Join<Left, Right, Inner>,
        > for $column_name where
            $column_name: AppearsOnTable<Join<Left, Right, Inner>>,
            Join<Left, Right, Inner>: AppearsInFromClause<$table, Count=Once>,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        impl<Join, On> SelectableExpression<JoinOn<Join, On>> for $column_name where
            $column_name: SelectableExpression<Join> + AppearsOnTable<JoinOn<Join, On>>,
        {
        }

        // FIXME: Remove this when overlapping marker traits are stable
        impl<From> SelectableExpression<SelectStatement<From>> for $column_name where
            $column_name: SelectableExpression<From> + AppearsOnTable<SelectStatement<From>>,
        {
        }

        impl $crate::expression::NonAggregate for $column_name {}

        impl $crate::query_source::Column for $column_name {
            type Table = $table;

            const NAME: &'static str = $sql_name;
        }

        impl<T> $crate::EqAll<T> for $column_name where
            T: $crate::expression::AsExpression<$($Type)*>,
            $crate::dsl::Eq<$column_name, T>: $crate::Expression<SqlType=$crate::sql_types::Bool>,
        {
            type Output = $crate::dsl::Eq<Self, T>;

            fn eq_all(self, rhs: T) -> Self::Output {
                $crate::expression::operators::Eq::new(self, rhs.as_expression())
            }
        }

        __diesel_generate_ops_impls_if_numeric!($column_name, $($Type)*);
        __diesel_generate_ops_impls_if_date_time!($column_name, $($Type)*);
    }
}

/// Specifies that a table exists, and what columns it has. This will create a
/// new public module, with the same name, as the name of the table. In this
/// module, you'll find a unit struct named `table`, and a unit struct with the
/// names of each of the columns.
///
/// By default this allows a maximum of 32 columns per table.
/// You can increase this limit to 64 by enabling the `64-column-tables` feature.
/// You can increase it to 128 by enabling the `128-column-tables` feature.
/// (The leading `x` is due to a bug in crates.io and will be removed in a
/// future release)
/// You can decrease it to 16 columns,
/// which improves compilation time,
/// by disabling the default features of Diesel.
/// Note that enabling 64 column tables or larger will substantially increase
/// the compile time of Diesel.
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
/// Tables with no primary key are not supported.
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
/// which types to import.
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
///     use diesel::sql_types::*;
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
/// If you want to add documentation to the generated code you can use the
/// following syntax:
///
/// ```
/// #[macro_use] extern crate diesel;
///
/// table! {
///
///     /// The table containing all blog posts
///     posts {
///         /// The post's unique id
///         id -> Integer,
///         /// The post's title
///         title -> Text,
///     }
/// }
/// # fn main() {}
/// ```
///
/// If you have a column with the same name as a Rust reserved keyword, you can use
/// the `sql_name` attribute like this:
///
/// ```
/// #[macro_use] extern crate diesel;
///
/// table! {
///     posts {
///         id -> Integer,
///         /// This column is named `mytype` but references the table `type` column.
///         #[sql_name = "type"]
///         mytype -> Text,
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
/// `all_columns`
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
/// `SqlType`
/// -------
///
/// A type alias called `SqlType` will be created. It will be the SQL type of
/// `all_columns`. The SQL type is needed for things like [returning boxed
/// queries][boxed_queries].
///
/// [boxed_queries]: query_dsl/trait.QueryDsl.html#method.into_boxed
///
/// `BoxedQuery`
/// ----------
///
/// ```ignore
/// pub type BoxedQuery<'a, DB, ST = SqlType> = BoxedSelectStatement<'a, ST, table, DB>;
/// ```
#[macro_export]
macro_rules! table {
    ($($tokens:tt)*) => {
        __diesel_parse_table! {
            tokens = [$($tokens)*],
            imports = [],
            meta = [],
            sql_name = unknown,
            name = unknown,
            schema = public,
            primary_key = (id),
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_invalid_table_syntax {
    () => {
        compile_error!(
            "Invalid `table!` syntax. Please see the `table!` macro docs for more info. \
             `https://docs.diesel.rs/diesel/macro.table.html`"
        );
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_parse_table {
    // Found an import
    (
        tokens = [use $($import:tt)::+; $($rest:tt)*],
        imports = [$($imports:tt)*],
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [$($rest)*],
            imports = [$($imports)* use $($import)::+;],
            $($args)*
        }
    };

    // Found sql_name attribute, override whatever we had before
    (
        tokens = [#[sql_name = $sql_name:expr] $($rest:tt)*],
        imports = $imports:tt,
        meta = $meta:tt,
        sql_name = $ignore:tt,
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [$($rest)*],
            imports = $imports,
            meta = $meta,
            sql_name = $sql_name,
            $($args)*
        }
    };

    // Meta item other than sql_name, attach it to the table struct
    (
        tokens = [#$new_meta:tt $($rest:tt)*],
        imports = $imports:tt,
        meta = [$($meta:tt)*],
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [$($rest)*],
            imports = $imports,
            meta = [$($meta)* #$new_meta],
            $($args)*
        }
    };

    // Found a schema name, override whatever we had before
    (
        tokens = [$schema:ident . $($rest:tt)*],
        imports = $imports:tt,
        meta = $meta:tt,
        sql_name = $sql_name:tt,
        name = $name:tt,
        schema = $ignore:tt,
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [$($rest)*],
            imports = $imports,
            meta = $meta,
            sql_name = $sql_name,
            name = $name,
            schema = $schema,
            $($args)*
        }
    };

    // Found a table name, override whatever we had before
    (
        tokens = [$name:ident $($rest:tt)*],
        imports = $imports:tt,
        meta = $meta:tt,
        sql_name = $sql_name:tt,
        name = $ignore:tt,
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [$($rest)*],
            imports = $imports,
            meta = $meta,
            sql_name = $sql_name,
            name = $name,
            $($args)*
        }
    };

    // Found a primary key, override whatever we had before
    (
        tokens = [($($pk:ident),+ $(,)*) $($rest:tt)*],
        imports = $imports:tt,
        meta = $meta:tt,
        sql_name = $sql_name:tt,
        name = $name:tt,
        schema = $schema:tt,
        primary_key = $ignore:tt,
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [$($rest)*],
            imports = $imports,
            meta = $meta,
            sql_name = $sql_name,
            name = $name,
            schema = $schema,
            primary_key = ($($pk),+),
            $($args)*
        }
    };

    // Reached columns with no imports, set a default
    (
        tokens = [{$($columns:tt)*}],
        imports = [],
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [{$($columns)*}],
            imports = [use $crate::sql_types::*;],
            $($args)*
        }
    };

    // Reached columns with no sql_name, set a default
    (
        tokens = [{$($columns:tt)*}],
        imports = $imports:tt,
        meta = $meta:tt,
        sql_name = unknown,
        name = $name:tt,
        $($args:tt)*
    ) => {
        __diesel_parse_table! {
            tokens = [{$($columns)*}],
            imports = $imports,
            meta = $meta,
            sql_name = stringify!($name),
            name = $name,
            $($args)*
        }
    };

    // Parse the columns
    (
        tokens = [{$($columns:tt)*}],
        $($args:tt)*
    ) => {
        __diesel_parse_columns! {
            tokens = [$($columns)*],
            table = { $($args)* },
            columns = [],
        }
    };

    // Invalid syntax
    ($($tokens:tt)*) => {
        __diesel_invalid_table_syntax!();
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_parse_columns {
    // No column being parsed, start a new one.
    // Attempt to capture the type as separate tokens if at all possible.
    (
        tokens = [
            $(#$meta:tt)*
            $name:ident -> $($ty:tt)::* $(<$($ty_params:tt)::*>)*,
            $($rest:tt)*
        ],
        $($args:tt)*
    ) => {
        __diesel_parse_columns! {
            current_column = {
                unchecked_meta = [$(#$meta)*],
                name = $name,
                sql_name = stringify!($name),
                ty = ($($ty)::* $(<$($ty_params)::*>)*),
                meta = [],
            },
            tokens = [$($rest)*],
            $($args)*
        }
    };

    // No column being parsed, start a new one. Couldn't keep the `ty` separate.
    (
        tokens = [
            $(#$meta:tt)*
            $name:ident -> $ty:ty,
            $($rest:tt)*
        ],
        $($args:tt)*
    ) => {
        __diesel_parse_columns! {
            current_column = {
                unchecked_meta = [$(#$meta)*],
                name = $name,
                sql_name = stringify!($name),
                ty = ($ty),
                meta = [],
            },
            tokens = [$($rest)*],
            $($args)*
        }
    };


    // Found #[sql_name]
    (
        current_column = {
            unchecked_meta = [#[sql_name = $sql_name:expr] $($meta:tt)*],
            name = $name:tt,
            sql_name = $ignore:expr,
            $($current_column:tt)*
        },
        $($args:tt)*
    ) => {
        __diesel_parse_columns! {
            current_column = {
                unchecked_meta = [$($meta)*],
                name = $name,
                sql_name = $sql_name,
                $($current_column)*
            },
            $($args)*
        }
    };

    // Meta item other than #[sql_name]
    (
        current_column = {
            unchecked_meta = [#$new_meta:tt $($unchecked_meta:tt)*],
            name = $name:tt,
            sql_name = $sql_name:expr,
            ty = $ty:tt,
            meta = [$($meta:tt)*],
            $($current_column:tt)*
        },
        $($args:tt)*
    ) => {
        __diesel_parse_columns! {
            current_column = {
                unchecked_meta = [$($unchecked_meta)*],
                name = $name,
                sql_name = $sql_name,
                ty = $ty,
                meta = [$($meta)* #$new_meta],
                $($current_column)*
            },
            $($args)*
        }
    };

    // Done parsing this column
    (
        current_column = {
            unchecked_meta = [],
            $($current_column:tt)*
        },
        tokens = $tokens:tt,
        table = $table:tt,
        columns = [$($columns:tt,)*],
        $($args:tt)*
    ) => {
        __diesel_parse_columns! {
            tokens = $tokens,
            table = $table,
            columns = [$($columns,)* { $($current_column)* },],
            $($args)*
        }
    };

    // Done parsing all columns
    (
        tokens = [],
        $($args:tt)*
    ) => {
        __diesel_table_impl!($($args)*);
    };

    // Invalid syntax
    ($($tokens:tt)*) => {
        __diesel_invalid_table_syntax!();
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_table_impl {
    (
        table = {
            imports = [$($imports:tt)*],
            meta = [$($meta:tt)*],
            sql_name = $sql_name:expr,
            name = $table_name:ident,
            schema = $schema:ident,
            primary_key = $primary_key:tt,
        },
        columns = [$({
            name = $column_name:ident,
            sql_name = $column_sql_name:expr,
            ty = ($($column_ty:tt)*),
            $($column:tt)*
        },)+],
    ) => {
        $($meta)*
        pub mod $table_name {
            #![allow(dead_code)]
            use $crate::{
                QuerySource,
                Table,
                JoinTo,
            };
            use $crate::associations::HasTable;
            use $crate::insertable::Insertable;
            use $crate::query_builder::*;
            use $crate::query_builder::nodes::Identifier;
            use $crate::query_source::{AppearsInFromClause, Once, Never};
            use $crate::query_source::joins::{Join, JoinOn};
            $($imports)*
            pub use self::columns::*;

            /// Re-exports all of the columns of this table, as well as the
            /// table struct renamed to the module name. This is meant to be
            /// glob imported for functions which only deal with one table.
            pub mod dsl {
                $(static_cond! {
                    if $table_name == $column_name {
                        compile_error!(concat!(
                            "Column `",
                            stringify!($column_name),
                            "` cannot be named the same as its table.\n \
                            You may use `#[sql_name = \"",
                            stringify!($column_name),
                            "\"]` to reference the table's `",
                            stringify!($column_name),
                            "` column. \n \
                            Docs available at: `https://docs.diesel.rs/diesel/macro.table.html`\n"
                        ));
                    } else {
                        pub use super::columns::{$column_name};
                    }
                })+
                pub use super::table as $table_name;
            }

            #[allow(non_upper_case_globals, dead_code)]
            /// A tuple of all of the columns on this table
            pub const all_columns: ($($column_name,)+) = ($($column_name,)+);

            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, QueryId)]
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
            pub type SqlType = ($($($column_ty)*,)+);

            /// Helper type for representing a boxed query from this table
            pub type BoxedQuery<'a, DB, ST = SqlType> = BoxedSelectStatement<'a, ST, table, DB>;

            __diesel_table_query_source_impl!(table, $schema, $sql_name);

            impl AsQuery for table {
                type SqlType = SqlType;
                type Query = SelectStatement<Self>;

                fn as_query(self) -> Self::Query {
                    SelectStatement::simple(self)
                }
            }

            impl Table for table {
                type PrimaryKey = $primary_key;
                type AllColumns = ($($column_name,)+);

                fn primary_key(&self) -> Self::PrimaryKey {
                    $primary_key
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

            impl AppearsInFromClause<table> for table {
                type Count = Once;
            }

            impl AppearsInFromClause<table> for () {
                type Count = Never;
            }

            impl<Left, Right, Kind> JoinTo<Join<Left, Right, Kind>> for table where
                Join<Left, Right, Kind>: JoinTo<table>,
            {
                type FromClause = Join<Left, Right, Kind>;
                type OnClause = <Join<Left, Right, Kind> as JoinTo<table>>::OnClause;

                fn join_target(rhs: Join<Left, Right, Kind>) -> (Self::FromClause, Self::OnClause) {
                    let (_, on_clause) = Join::join_target(table);
                    (rhs, on_clause)
                }
            }

            impl<Join, On> JoinTo<JoinOn<Join, On>> for table where
                JoinOn<Join, On>: JoinTo<table>,
            {
                type FromClause = JoinOn<Join, On>;
                type OnClause = <JoinOn<Join, On> as JoinTo<table>>::OnClause;

                fn join_target(rhs: JoinOn<Join, On>) -> (Self::FromClause, Self::OnClause) {
                    let (_, on_clause) = JoinOn::join_target(table);
                    (rhs, on_clause)
                }
            }

            impl<F, S, D, W, O, L, Of, G> JoinTo<SelectStatement<F, S, D, W, O, L, Of, G>> for table where
                SelectStatement<F, S, D, W, O, L, Of, G>: JoinTo<table>,
            {
                type FromClause = SelectStatement<F, S, D, W, O, L, Of, G>;
                type OnClause = <SelectStatement<F, S, D, W, O, L, Of, G> as JoinTo<table>>::OnClause;

                fn join_target(rhs: SelectStatement<F, S, D, W, O, L, Of, G>) -> (Self::FromClause, Self::OnClause) {
                    let (_, on_clause) = SelectStatement::join_target(table);
                    (rhs, on_clause)
                }
            }

            impl<'a, QS, ST, DB> JoinTo<BoxedSelectStatement<'a, QS, ST, DB>> for table where
                BoxedSelectStatement<'a, QS, ST, DB>: JoinTo<table>,
            {
                type FromClause = BoxedSelectStatement<'a, QS, ST, DB>;
                type OnClause = <BoxedSelectStatement<'a, QS, ST, DB> as JoinTo<table>>::OnClause;
                fn join_target(rhs: BoxedSelectStatement<'a, QS, ST, DB>) -> (Self::FromClause, Self::OnClause) {
                    let (_, on_clause) = BoxedSelectStatement::join_target(table);
                    (rhs, on_clause)
                }
            }

            // This impl should be able to live in Diesel,
            // but Rust tries to recurse for no reason
            impl<T> Insertable<T> for table
            where
                <table as AsQuery>::Query: Insertable<T>,
            {
                type Values = <<table as AsQuery>::Query as Insertable<T>>::Values;

                fn values(self) -> Self::Values {
                    self.as_query().values()
                }
            }

            impl<'a, T> Insertable<T> for &'a table
            where
                table: Insertable<T>,
            {
                type Values = <table as Insertable<T>>::Values;

                fn values(self) -> Self::Values {
                    (*self).values()
                }
            }

            /// Contains all of the columns of this table
            pub mod columns {
                use super::table;
                use $crate::{Expression, SelectableExpression, AppearsOnTable, QuerySource};
                use $crate::backend::Backend;
                use $crate::query_builder::{QueryFragment, AstPass, SelectStatement};
                use $crate::query_source::joins::{Join, JoinOn, Inner, LeftOuter};
                use $crate::query_source::{AppearsInFromClause, Once, Never};
                use $crate::result::QueryResult;
                $($imports)*

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
                    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
                        table.from_clause().walk_ast(out.reborrow())?;
                        out.push_sql(".*");
                        Ok(())
                    }
                }

                impl SelectableExpression<table> for star {
                }

                impl AppearsOnTable<table> for star {
                }

                $(__diesel_column! {
                    table = table,
                    name = $column_name,
                    sql_name = $column_sql_name,
                    ty = ($($column_ty)*),
                    $($column)*
                })+
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_table_query_source_impl {
    ($table_struct:ident, public, $table_name:expr) => {
        impl QuerySource for $table_struct {
            type FromClause = Identifier<'static>;
            type DefaultSelection = <Self as Table>::AllColumns;

            fn from_clause(&self) -> Self::FromClause {
                Identifier($table_name)
            }

            fn default_selection(&self) -> Self::DefaultSelection {
                Self::all_columns()
            }
        }
    };

    ($table_struct:ident, $schema_name:ident, $table_name:expr) => {
        impl QuerySource for $table_struct {
            type FromClause = $crate::query_builder::nodes::InfixNode<
                'static,
                Identifier<'static>,
                Identifier<'static>,
            >;
            type DefaultSelection = <Self as Table>::AllColumns;

            fn from_clause(&self) -> Self::FromClause {
                $crate::query_builder::nodes::InfixNode::new(
                    Identifier(stringify!($schema_name)),
                    Identifier($table_name),
                    ".",
                )
            }

            fn default_selection(&self) -> Self::DefaultSelection {
                Self::all_columns()
            }
        }
    };
}

/// Allow two tables to be referenced in a join query without providing an
/// explicit `ON` clause.
///
/// The generated `ON` clause will always join to the primary key of the parent
/// table. This macro removes the need to call [`.on`] explicitly, you will
/// still need to invoke [`allow_tables_to_appear_in_same_query!`] for these two tables to
/// be able to use the resulting query, unless you are using `infer_schema!` or
/// `diesel print-schema` which will generate it for you.
///
/// If you are using `infer_schema!` or `diesel print-schema`, an invocation of
/// this macro will be generated for every foreign key in your database unless
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
/// # #[macro_use] extern crate diesel;
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
/// So given the Table decaration from [Associations docs](http://docs.diesel.rs/diesel/associations/index.html)
///
/// * The parent table would be `User`
/// * The child table would be `Post`
/// * and the Foreign key would be `Post.user_id`
///
/// For joins that do not explicitly use on clauses via [`JoinOnDsl`](http://docs.diesel.rs/diesel/prelude/trait.JoinOnDsl.html)
/// the following on clause is generated implicitly:
/// ```sql
/// post JOIN users ON posts.user_id = users.id
/// ```
#[macro_export]
macro_rules! joinable {
    ($($child:ident)::* -> $($parent:ident)::* ($source:ident)) => {
        joinable_inner!($($child)::* ::table => $($parent)::* ::table : ($($child)::* ::$source = $($parent)::* ::table));
        joinable_inner!($($parent)::* ::table => $($child)::* ::table : ($($child)::* ::$source = $($parent)::* ::table));
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
                $crate::expression::nullable::Nullable<$foreign_key>,
                $crate::expression::nullable::Nullable<$primary_key_ty>,
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
/// If you are using `infer_schema!` or `diesel print-schema`, an invocation of
/// this macro will be generated for you for all tables in your schema.
///
/// # Example
///
/// ```ignore
/// // This would be required to do `users.inner_join(posts.inner_join(comments))`
/// allow_tables_to_appear_in_same_query!(comments, posts, users);
/// ```
///
/// When more than two tables are passed, the relevant code is generated for
/// every combination of those tables. This code would be equivalent to the
/// previous example.
///
/// ```ignore
/// allow_tables_to_appear_in_same_query!(comments, posts);
/// allow_tables_to_appear_in_same_query!(comments, users);
/// allow_tables_to_appear_in_same_query!(posts, users);
/// ```
#[macro_export]
macro_rules! allow_tables_to_appear_in_same_query {
    ($($table_mod:ident),* $(,)*) => {
        allow_types_to_appear_in_same_query!($($table_mod::table,)*);
    };
}

#[macro_export]
macro_rules! allow_types_to_appear_in_same_query {
    ($left_ty:ty, $($right_ty:ty),+ $(,)*) => {
        $(
            impl $crate::query_source::AppearsInFromClause<$left_ty>
                for $right_ty
            {
                type Count = $crate::query_source::Never;
            }

            impl $crate::query_source::AppearsInFromClause<$right_ty>
                for $left_ty
            {
                type Count = $crate::query_source::Never;
            }
        )+
        allow_types_to_appear_in_same_query!($($right_ty,)+);
    };

    ($last_ty:ty,) => {};

    () => {};
}

#[macro_export]
#[doc(hidden)]
/// Used by `diesel_derives`, which can't access `$crate`
macro_rules! __diesel_use_everything {
    () => {
        pub use $crate::*;
    };
}

/// Gets the value out of an option, or returns an error.
///
/// This is used by `FromSql` implementations.
#[macro_export]
macro_rules! not_none {
    ($bytes:expr) => {
        match $bytes {
            Some(bytes) => bytes,
            None => return Err(Box::new($crate::result::UnexpectedNullError)),
        }
    };
}

// The order of these modules is important (at least for those which have tests).
// Utility macros which don't call any others need to come first.
#[macro_use]
mod internal;
#[macro_use]
#[cfg(diesel_experimental)]
mod aliasing;
#[macro_use]
mod query_id;
#[macro_use]
mod static_cond;
#[macro_use]
mod ops;
#[macro_use]
mod tuples;

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
        #[derive(Debug, Clone, Copy)]
        pub struct MyCustomType;
    }

    table! {
        use sql_types::*;
        use macros::tests::my_types::*;

        table_with_custom_types {
            id -> Integer,
            my_type -> MyCustomType,
        }
    }

    table! {
        use sql_types::*;
        use macros::tests::my_types::*;

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
        use pg::Pg;
        let expected_sql = r#"SELECT "foo"."bars"."baz" FROM "foo"."bars" -- binds: []"#;
        assert_eq!(
            expected_sql,
            &::debug_query::<Pg, _>(&bars::table.select(bars::baz)).to_string()
        );
    }

    table! {
        use sql_types;
        use sql_types::*;

        table_with_arbitrarily_complex_types {
            id -> sql_types::Integer,
            qualified_nullable -> sql_types::Nullable<sql_types::Integer>,
            deeply_nested_type -> Option<Nullable<Integer>>,
            // This actually should work, but there appears to be a rustc bug
            // on the `AsExpression` bound for `EqAll` when the ty param is a projection
            // projected_type -> <Nullable<Integer> as sql_types::IntoNullable>::Nullable,
            random_tuple -> (Integer, Integer),
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
        use pg::Pg;
        let expected_sql =
            r#"SELECT "foo"."id", "foo"."type", "foo"."bleh" FROM "foo" WHERE "foo"."type" = $1 -- binds: [1]"#;
        assert_eq!(
            expected_sql,
            ::debug_query::<Pg, _>(&foo::table.filter(foo::mytype.eq(1))).to_string()
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn table_with_column_renaming_mysql() {
        use mysql::Mysql;
        let expected_sql =
            r#"SELECT `foo`.`id`, `foo`.`type`, `foo`.`bleh` FROM `foo` WHERE `foo`.`type` = ? -- binds: [1]"#;
        assert_eq!(
            expected_sql,
            ::debug_query::<Mysql, _>(&foo::table.filter(foo::mytype.eq(1))).to_string()
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn table_with_column_renaming_sqlite() {
        use sqlite::Sqlite;
        let expected_sql =
            r#"SELECT `foo`.`id`, `foo`.`type`, `foo`.`bleh` FROM `foo` WHERE `foo`.`type` = ? -- binds: [1]"#;
        assert_eq!(
            expected_sql,
            ::debug_query::<Sqlite, _>(&foo::table.filter(foo::mytype.eq(1))).to_string()
        );
    }

    table!(
        use sql_types::*;

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
        use pg::Pg;
        let expected_sql = r#"SELECT "mod"."id" FROM "mod" -- binds: []"#;
        assert_eq!(
            expected_sql,
            ::debug_query::<Pg, _>(&bar::table.select(bar::id)).to_string()
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn table_renaming_mysql() {
        use mysql::Mysql;
        let expected_sql = r#"SELECT `mod`.`id` FROM `mod` -- binds: []"#;
        assert_eq!(
            expected_sql,
            ::debug_query::<Mysql, _>(&bar::table.select(bar::id)).to_string()
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn table_renaming_sqlite() {
        use sqlite::Sqlite;
        let expected_sql = r#"SELECT `mod`.`id` FROM `mod` -- binds: []"#;
        assert_eq!(
            expected_sql,
            ::debug_query::<Sqlite, _>(&bar::table.select(bar::id)).to_string()
        );
    }
}
