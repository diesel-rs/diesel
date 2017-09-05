#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_column {
    ($($table:ident)::*, $column_name:ident -> ($($Type:tt)*),  $sql_name:expr, $($doc:expr),*) => {
        $(
            #[doc=$doc]
        )*
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Debug, Clone, Copy)]
        pub struct $column_name;

        impl $crate::expression::Expression for $column_name {
            type SqlType = $($Type)*;
        }

        impl<DB> $crate::query_builder::QueryFragment<DB> for $column_name where
            DB: $crate::backend::Backend,
            <$($table)::* as QuerySource>::FromClause: QueryFragment<DB>,
        {
            fn walk_ast(&self, mut out: $crate::query_builder::AstPass<DB>) -> $crate::result::QueryResult<()> {
                $($table)::*.from_clause().walk_ast(out.reborrow())?;
                out.push_sql(".");
                out.push_identifier($sql_name)
            }
        }

        impl_query_id!($column_name);

        impl SelectableExpression<$($table)::*> for $column_name {
        }

        impl<QS> AppearsOnTable<QS> for $column_name where
            QS: AppearsInFromClause<$($table)::*, Count=Once>,
        {
        }

        impl<Left, Right> SelectableExpression<
            Join<Left, Right, LeftOuter>,
        > for $column_name where
            $column_name: AppearsOnTable<Join<Left, Right, LeftOuter>>,
            Left: AppearsInFromClause<$($table)::*, Count=Once>,
            Right: AppearsInFromClause<$($table)::*, Count=Never>,
        {
        }

        impl<Left, Right> SelectableExpression<
            Join<Left, Right, Inner>,
        > for $column_name where
            $column_name: AppearsOnTable<Join<Left, Right, Inner>>,
            Join<Left, Right, Inner>: AppearsInFromClause<$($table)::*, Count=Once>,
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
            type Table = $($table)::*;

            const NAME: &'static str = $sql_name;
        }

        impl<T> $crate::EqAll<T> for $column_name where
            T: $crate::expression::AsExpression<$($Type)*>,
            $crate::dsl::Eq<$column_name, T>: $crate::Expression<SqlType=$crate::types::Bool>,
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
/// [boxed_queries]: prelude/trait.BoxedDsl.html#example-1
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
        __diesel_table_impl!($($tokens)*);
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_table_impl {
    // Put imports into the import field
    (
        @parse
        import = [$(use $($import:tt)::+;)*];
        table_doc = [];
        use $($new_import:tt)::+; $($rest:tt)+
    ) => {
        table! {
            @parse
            import = [$(use $($import)::+;)* use $($new_import)::+;];
            table_doc = [];
            $($rest)+
        }
    };

    // Put doc annotation into the doc field
    (
        @parse
        import = [$(use $($import:tt)::+;)*];
        table_doc = [$($doc:expr,)*];
        #[doc=$new_doc:expr] $($rest:tt)+
    ) => {
        table! {
            @parse
            import = [$(use $($import)::+;)*];
            table_doc = [$($doc,)*$new_doc,];
            $($rest)+
        }
    };

    // We are finished parsing the import list and the table documentation
    // Now we forward the remaining tokens to parse the body of the table
    // definition
    (
        @parse
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($doc:expr,)*];
        $($rest:tt)+
    ) => {
        table! {
            @parse_body
            import = [$(use $($import)::+;)*];
            table_doc = [$($doc,)*];
            $($rest)+
        }
    };

    // We are finished parsing the import list and the table documentation
    // Because the import list is empty we add a default import (diesel::types::*)
    // After that we forward the remaining tokens to parse the body of the table
    // definition
    (
        @parse
        import = [];
        table_doc = [$($doc:expr,)*];
        $($rest:tt)+
    ) => {
        table! {
            @parse_body
            import = [use $crate::types::*;];
            table_doc = [$($doc,)*];
            $($rest)+
        }
    };

    // Add the primary key if it's not present
    (
        @parse_body
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($doc:expr,)*];
        $($table_name:ident).+ {$($body:tt)*}
    ) => {
        table! {
            @parse_body
            import = [$(use $($import)::+;)+];
            table_doc = [$($doc,)*];
            $($table_name).+ (id) {$($body)*}
        }
    };

    // Add the schema name if it's not present
    (
        @parse_body
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($doc:expr,)*];
        $name:ident $(($($pk:ident),+))* {$($body:tt)*}
    ) => {
        table! {
            @parse_body
            import = [$(use $($import)::+;)+];
            table_doc = [$($doc,)*];
            public . $name $(($($pk),+))* {$($body)*}
        }
    };

    // Terminal with single-column pk
    (
        @parse_body
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($doc:expr,)*];
        $schema_name:ident . $name:ident ($pk:ident) $body:tt
    ) => {
        table_body! {
            $schema_name . $name ($pk) $body
            import = [$(use $($import)::+;)+];
            table_doc = [$($doc)*];
        }
    };

    // Terminal with composite pk (add a trailing comma)
    (
        @parse_body
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($doc:expr,)*];
        $schema_name:ident . $name:ident ($pk:ident, $($composite_pk:ident),+) $body:tt
    ) => {
        table_body! {
            $schema_name . $name ($pk, $($composite_pk,)+) $body
            import = [$(use $($import)::+;)+];
            table_doc = [$($doc)*];
        }
    };

    // Terminal with invalid syntax
    // This is needed to prevent unbounded recursion on for example
    // table! {
    //     something strange
    // }
    (
        @parse_body
        import = [$(use $($import:tt)::+;)*];
        table_doc = [$($doc:expr,)*];
        $($rest:tt)*
    ) => {
        compile_error!("Invalid `table!` syntax. Please see the `table!` macro docs for more info. \
        `http://docs.diesel.rs/diesel/macro.table.html`");
    };

    // Put a parse annotation and empty fields for imports and documentation on top
    // This is the entry point for parsing the table dsl
    ($($rest:tt)+) => {
        table! {
            @parse
            import = [];
            table_doc = [];
            $($rest)+
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! table_body {
    // Parse the documentation of a table column and store it in current_column_doc
    // Forward the remaining table body to further instances of this macro
    (
        schema_name = $schema_name:ident,
        table_name = $name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $Type:tt; doc = [$($doc:expr)*]; sql_name = $sql_name:expr,)*],
        imports = ($($($import:tt)::+),+),
        table_doc = [$($table_doc:expr)*],
        current_column_doc = [$($column_doc:expr)*],
        current_column_sql_name = [$($current_column_sql_name:expr)*],
        #[doc=$new_doc:expr]
        $($body:tt)*
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = $primary_key_ty,
            primary_key_expr = $primary_key_expr,
            columns = [$($column_name -> $Type; doc = [$($doc)*]; sql_name = $sql_name,)*],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [$($column_doc)*$new_doc],
            current_column_sql_name = [$($current_column_sql_name)*],
            $($body)*
        }
    };

    // Parse the sql_name attribute and forward the remaining table body to further instances of
    // this macro
    (
        schema_name = $schema_name:ident,
        table_name = $name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $Type:tt; doc = [$($doc:expr)*]; sql_name = $sql_name:expr,)*],
        imports = ($($($import:tt)::+),+),
        table_doc = [$($table_doc:expr)*],
        current_column_doc = [$($column_doc:expr)*],
        current_column_sql_name = [],
        #[sql_name=$new_sql_name:expr]
        $($body:tt)*
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = $primary_key_ty,
            primary_key_expr = $primary_key_expr,
            columns = [$($column_name -> $Type; doc = [$($doc)*]; sql_name = $sql_name,)*],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [$($column_doc)*],
            current_column_sql_name = [$new_sql_name],
            $($body)*
        }
    };

    // Parse a table column definition
    // Forward any remaining table column to further instances
    // of this macro
    //
    // This case will attempt to keep the type destructured so we can match
    // on it to determine if the column is numeric, later. The next branch
    // will catch any types which don't match this structure
    (
        schema_name = $schema_name:ident,
        table_name = $name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $Type:tt; doc = [$($doc:expr)*]; sql_name = $sql_name:expr,)*],
        imports = ($($($import:tt)::+),+),
        table_doc = [$($table_doc:expr)*],
        current_column_doc = [$($column_doc:expr)*],
        current_column_sql_name = [$new_sql_name:expr],
        $new_column_name:ident -> $($ty_path:tt)::* $(<$($ty_params:tt)::*>)*,
        $($body:tt)*
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = $primary_key_ty,
            primary_key_expr = $primary_key_expr,
            columns = [$($column_name -> $Type; doc = [$($doc)*]; sql_name = $sql_name,)*
                       $new_column_name -> ($($ty_path)::*$(<$($ty_params)::*>)*); doc = [$($column_doc)*]; sql_name = $new_sql_name,],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [],
            current_column_sql_name = [],
            $($body)*
        }
    };

    // Parse a table column definition with a complex type
    //
    // This is identical to the previous branch, but we are capturing the whole
    // thing as a `ty` token.
    (
        schema_name = $schema_name:ident,
        table_name = $name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $Type:tt; doc = [$($doc:expr)*]; sql_name = $sql_name:expr,)*],
        imports = ($($($import:tt)::+),+),
        table_doc = [$($table_doc:expr)*],
        current_column_doc = [$($column_doc:expr)*],
        current_column_sql_name = [$new_sql_name:expr],
        $new_column_name:ident -> $new_column_ty:ty,
        $($body:tt)*
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = $primary_key_ty,
            primary_key_expr = $primary_key_expr,
            columns = [$($column_name -> $Type; doc = [$($doc)*]; sql_name = $sql_name,)*
                       $new_column_name -> ($new_column_ty); doc = [$($column_doc)*]; sql_name = $new_sql_name,],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [],
            current_column_sql_name = [],
            $($body)*
        }
    };

    // Parse the table name  and the primary keys
    // Forward the table body to further parsing layers that parses
    // the column definitions
    (
        $schema_name:ident . $name:ident ($pk:ident) {
            $($body:tt)+
        }
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($table_doc:expr)*];
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = columns::$pk,
            primary_key_expr = columns::$pk,
            columns = [],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [],
            current_column_sql_name = [],
            $($body)+
        }
    };

    // Add a sql_name arg if we find a column definition without any sql_name attribute before
    (
        schema_name = $schema_name:ident,
        table_name = $name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> $Type:tt; doc = [$($doc:expr)*]; sql_name = $sql_name:expr,)*],
        imports = ($($($import:tt)::+),+),
        table_doc = [$($table_doc:expr)*],
        current_column_doc = [$($column_doc:expr)*],
        current_column_sql_name = [],
        $new_column_name:ident ->
        $($body:tt)*
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = $primary_key_ty,
            primary_key_expr = $primary_key_expr,
            columns = [$($column_name -> $Type; doc = [$($doc)*]; sql_name = $sql_name,)*],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [$($column_doc)*],
            current_column_sql_name = [stringify!($new_column_name)],
            $new_column_name -> $($body)*
        }
    };

    (
        $schema_name:ident . $name:ident ($($pk:ident,)+) {
            $($body:tt)+
        }
        import = [$(use $($import:tt)::+;)+];
        table_doc = [$($table_doc:expr)*];
    ) => {
        table_body! {
            schema_name = $schema_name,
            table_name = $name,
            primary_key_ty = ($(columns::$pk,)+),
            primary_key_expr = ($(columns::$pk,)+),
            columns = [],
            imports = ($($($import)::+),+),
            table_doc = [$($table_doc)*],
            current_column_doc = [],
            current_column_sql_name = [],
            $($body)+
        }
    };

    // Finish parsing the table dsl. Now expand the parsed informations into
    // the corresponding rust code
    (
        schema_name = $schema_name:ident,
        table_name = $table_name:ident,
        primary_key_ty = $primary_key_ty:ty,
        primary_key_expr = $primary_key_expr:expr,
        columns = [$($column_name:ident -> ($($column_ty:tt)*); doc = [$($doc:expr)*]; sql_name = $sql_name:expr,)+],
        imports = ($($($import:tt)::+),+),
        table_doc = [$($table_doc:expr)*],
        current_column_doc = [],
        current_column_sql_name = [],
    ) => {
        $(
            #[doc=$table_doc]
        )*
        pub mod $table_name {
            #![allow(dead_code)]
            use $crate::{
                QuerySource,
                Table,
                JoinTo,
            };
            use $crate::associations::HasTable;
            use $crate::query_builder::*;
            use $crate::query_builder::nodes::Identifier;
            use $crate::query_source::{AppearsInFromClause, Once, Never};
            use $crate::query_source::joins::{Join, JoinOn};
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
            pub type SqlType = ($($($column_ty)*,)+);

            /// Helper type for representing a boxed query from this table
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

            impl AppearsInFromClause<table> for table {
                type Count = Once;
            }

            impl<T> AppearsInFromClause<T> for table where
                T: Table + JoinTo<table>,
            {
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

            impl_query_id!(table);

            /// Contains all of the columns of this table
            pub mod columns {
                use super::table;
                use $crate::{Expression, SelectableExpression, AppearsOnTable, QuerySource};
                use $crate::backend::Backend;
                use $crate::query_builder::{QueryFragment, AstPass, SelectStatement};
                use $crate::query_source::joins::{Join, JoinOn, Inner, LeftOuter};
                use $crate::query_source::{AppearsInFromClause, Once, Never};
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

                $(__diesel_column!(table, $column_name -> ($($column_ty)*), $sql_name, $($doc),*);)+
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

/// Allow two tables to be referenced in a join query.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # use diesel::prelude::*;
/// mod schema {
///    table! {
///         users(id) {
///             id -> Integer,
///         }
///     }
///     table! {
///         posts(id) {
///             id -> Integer,
///             user_id -> Integer,
///         }
///     }
///
/// }
///
/// joinable!(schema::posts -> schema::users(user_id));
///
/// # fn main() {
/// use schema::*;
///
/// // Without the joinable! call, this wouldn't compile
/// let query = users::table.inner_join(posts::table);
/// # }
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
            primary_key_expr = <$parent_table as $crate::query_source::Table>::primary_key(&$parent_table),
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

                (rhs, $foreign_key.nullable().eq($primary_key_expr.nullable()))
            }
        }
    }
}

/// Allow two tables which are otherwise unrelated to be used together in a
/// multi-table join. This macro only needs to be invoked when the two tables
/// don't have an association between them (e.g. parent to grandchild)
///
/// # Example
///
/// ```ignore
/// // This would be required to do `users.inner_join(posts.inner_join(comments))`
/// // if there were an association between users and posts, and an association
/// // between posts and comments, but no association between users and comments
/// enable_multi_table_joins!(users, comments);
/// ```
#[macro_export]
macro_rules! enable_multi_table_joins {
    ($left_mod:ident, $right_mod:ident) => {
        impl $crate::query_source::AppearsInFromClause<$left_mod::table>
            for $right_mod::table
        {
            type Count = $crate::query_source::Never;
        }

        impl $crate::query_source::AppearsInFromClause<$right_mod::table>
            for $left_mod::table
        {
            type Count = $crate::query_source::Never;
        }
    }
}

/// Takes a query `QueryFragment` expression as an argument and returns a string
/// of SQL with placeholders for the dynamic values.
///
/// # Example
///
/// ### Returning SQL from a count statement:
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
/// if cfg!(feature = "postgres") {
///     assert_eq!(sql, r#"SELECT COUNT(*) FROM "users" -- binds: []"#);
/// } else {
///     assert_eq!(sql, "SELECT COUNT(*) FROM `users` -- binds: []");
/// }
///
/// let sql = debug_sql!(users.filter(n.eq(1)));
/// if cfg!(feature = "postgres") {
///     assert_eq!(sql, r#"SELECT "users"."id", "users"."n" FROM "users" WHERE "users"."n" = $1 -- binds: [1]"#);
/// } else {
///     assert_eq!(sql, "SELECT `users`.`id`, `users`.`n` FROM `users` WHERE \
///         `users`.`n` = ? -- binds: [1]");
/// }
/// # }
/// ```
#[cfg(feature = "with-deprecated")]
#[macro_export]
#[deprecated(since = "0.16.0", note = "use `diesel::debug_query(...).to_string()` instead")]
macro_rules! debug_sql {
    ($query:expr) => {
        $crate::query_builder::deprecated_debug_sql(&$query)
    };
}

/// Takes takes a query `QueryFragment` expression as an argument and prints out
/// the SQL with placeholders for the dynamic values.
///
/// # Example
///
/// ### Printing SQL from a count statement:
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
#[cfg(feature = "with-deprecated")]
#[deprecated(since = "0.16.0", note = "use `println!(\"{}\", diesel::debug_query(...))` instead")]
macro_rules! print_sql {
    ($query:expr) => {
        println!("{}", debug_sql!($query));
    };
}

// The order of these modules is important (at least for those which have tests).
// Utililty macros which don't call any others need to come first.
#[macro_use]
mod internal;
#[macro_use]
mod parse;
#[macro_use]
mod query_id;
#[macro_use]
mod static_cond;
#[macro_use]
mod macros_from_codegen;
#[macro_use]
mod ops;

#[macro_use]
mod as_changeset;
#[macro_use]
mod associations;
#[macro_use]
mod identifiable;
#[macro_use]
mod insertable;

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

    table! {
        use types::*;
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
        use types;
        use types::*;

        table_with_arbitrarily_complex_types {
            id -> types::Integer,
            qualified_nullable -> types::Nullable<types::Integer>,
            deeply_nested_type -> Option<Nullable<Integer>>,
            // This actually should work, but there appears to be a rustc bug
            // on the `AsExpression` bound for `EqAll` when the ty param is a projection
            // projected_type -> <Nullable<Integer> as types::IntoNullable>::Nullable,
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
        let expected_sql = r#"SELECT "foo"."id", "foo"."type", "foo"."bleh" FROM "foo" WHERE "foo"."type" = $1 -- binds: [1]"#;
        assert_eq!(expected_sql, ::debug_query::<Pg, _>(&foo::table.filter(foo::mytype.eq(1))).to_string());
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn table_with_column_renaming_mysql() {
        use mysql::Mysql;
        let expected_sql = r#"SELECT `foo`.`id`, `foo`.`type`, `foo`.`bleh` FROM `foo` WHERE `foo`.`type` = ? -- binds: [1]"#;
        assert_eq!(expected_sql, ::debug_query::<Mysql, _>(&foo::table.filter(foo::mytype.eq(1))).to_string());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn table_with_column_renaming_sqlite() {
        use sqlite::Sqlite;
        let expected_sql = r#"SELECT `foo`.`id`, `foo`.`type`, `foo`.`bleh` FROM `foo` WHERE `foo`.`type` = ? -- binds: [1]"#;
        assert_eq!(expected_sql, ::debug_query::<Sqlite, _>(&foo::table.filter(foo::mytype.eq(1))).to_string());
    }
}
