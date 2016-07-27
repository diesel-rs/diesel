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

        joinable_inner!(
            left_table_ty = $child_table_name::table,
            right_table_ty = <$parent_struct as $crate::associations::Identifiable>::Table,
            right_table_expr = <$parent_struct as $crate::associations::Identifiable>::table(),
            foreign_key = $child_table_name::$foreign_key_name,
            primary_key_ty = <<$parent_struct as $crate::associations::Identifiable>::Table as $crate::Table>::PrimaryKey,
            primary_key_expr = $crate::Table::primary_key(&<$parent_struct as $crate::associations::Identifiable>::table()),
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
