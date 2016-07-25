/// Defines a one-to-one association for the child table. This macro should be
/// called with the name of the parent struct, followed by any options, followed
/// by the entire struct body. The struct *must* be annotated with
/// `#[table_name(name_of_table)]`. Both the parent and child structs must
/// implement [`Identifiable`][identifiable].
///
/// [identifiable]: prelude/trait.Identifiable.html
///
/// # Options
///
/// ## foreign_key
///
/// Required. The name of the foreing key column for this association.
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # table! { users { id -> Integer, } }
/// # table! { posts { id -> Integer, user_id -> Integer, } }
/// pub struct User {
///     id: i32,
/// }
/// # Identifiable! { #[table_name(users)] struct User { id: i32, } }
///
/// pub struct Post {
///     id: i32,
///     user_id: i32,
/// }
/// # Identifiable! { #[table_name(posts)] struct Post { id: i32, user_id: i32, } }
///
/// BelongsTo! {
///     (User, foreign_key = user_id)
///     #[table_name(posts)]
///     struct Post {
///         id: i32,
///         user_id: i32,
///     }
/// }
/// # fn main() {}
/// ```
///
/// To avoid copying your struct definition, you can use the
/// [custom_derive crate][custom_derive].
///
/// [custom_derive]: https://crates.io/crates/custom_derive
///
/// ```ignore
/// custom_derive! {
///     #[derive(BelongsTo(User, foreign_key = user_id)]
///     #[table_name(posts)]
///     struct Post {
///         id: i32,
///         user_id: i32,
///     }
/// }
/// ```
///
/// This macro cannot be used with tuple structs.
#[macro_export]
macro_rules! BelongsTo {
    // Format arguments
    (
        ($parent_struct:ident, foreign_key = $foreign_key_name:ident)
        $($rest:tt)*
    ) => {
        BelongsTo! {
            (
                parent_struct = $parent_struct,
                foreign_key_name = $foreign_key_name,
            )
            $($rest)*
        }
    };

    // Extract table name from struct
    (
        ($($args:tt)*)
        #[table_name($table_name:ident)]
        $($rest:tt)*
    ) => {
        BelongsTo! {
            (
                $($args)*
                child_table_name = $table_name,
            )
            $($rest)*
        }
    };

    // Strip meta items, pub (if present) and struct from definition
    (
        $args:tt
        $(#[$ignore:meta])*
        $(pub)* struct $($body:tt)*
    ) => {
        BelongsTo! {
            $args
            $($body)*
        }
    };

    // Receive parsed fields of normal struct from `__diesel_parse_struct_body`
    // These patterns must appear above those which start with an ident to compile
    (
        (
            struct_name = $struct_name:ident,
            parent_struct = $parent_struct:ident,
            foreign_key_name = $foreign_key_name:ident,
            child_table_name = $child_table_name:ident,
        ),
        fields = [$({
            field_name: $field_name:ident,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
        })+],
    ) => {
        impl $crate::associations::BelongsTo<$parent_struct> for $struct_name {
            type ForeignKeyColumn = $child_table_name::$foreign_key_name;

            fn foreign_key(&self) -> <$parent_struct as $crate::associations::Identifiable>::Id {
                self.$foreign_key_name
            }

            fn foreign_key_column() -> Self::ForeignKeyColumn {
                $child_table_name::$foreign_key_name
            }
        }

        __diesel_belongs_to_joinable_impl!(
            parent_table_ty = <$parent_struct as $crate::associations::Identifiable>::Table,
            parent_table_expr = <$parent_struct as $crate::associations::Identifiable>::table(),
            child_table = $child_table_name::table,
            foreign_key = $child_table_name::$foreign_key_name,
        );

        $(select_column_inner!(
            $child_table_name::table,
            <$parent_struct as $crate::associations::Identifiable>::Table,
            $child_table_name::$column_name,
        );)+
        select_column_inner!(
            $child_table_name::table,
            <$parent_struct as $crate::associations::Identifiable>::Table,
            $child_table_name::star,
        );
    };

    // Handle struct with no generics
    (
        ($($args:tt)*)
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                $($args)*
            ),
            callback = BelongsTo,
            body = $body,
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_belongs_to_joinable_impl {
    (
        parent_table_ty = $parent_table_ty:ty,
        parent_table_expr = $parent_table_expr:expr,
        child_table = $child_table:path,
        foreign_key = $foreign_key:path,
    ) => {
        impl<JoinType> $crate::JoinTo<$parent_table_ty, JoinType> for $child_table {
            type JoinClause = $crate::query_builder::nodes::Join<
                <$child_table as $crate::QuerySource>::FromClause,
                <$parent_table_ty as $crate::QuerySource>::FromClause,
                $crate::expression::helper_types::Eq<
                    $crate::expression::nullable::Nullable<$foreign_key>,
                    $crate::expression::nullable::Nullable<
                        <$parent_table_ty as $crate::query_source::Table>::PrimaryKey>,
                >,
                JoinType,
            >;

            fn join_clause(&self, join_type: JoinType) -> Self::JoinClause {
                use $crate::{QuerySource, Table, ExpressionMethods};

                $crate::query_builder::nodes::Join::new(
                    self.from_clause(),
                    $parent_table_expr.from_clause(),
                    $foreign_key.nullable().eq($parent_table_expr.primary_key().nullable()),
                    join_type,
                )
            }
        }
    };
}
