/// Defines a one-to-many association for the parent table. This macro is only required if you need
/// to join between the two tables. This macro should be called with the name of the child table,
/// followed by any options, followed by the entire struct body. The struct *must* be annotated with
/// `#[table_name(name_of_table)]`. Both the parent and child structs must implement
/// [`Identifiable`][identifiable].
///
/// [identifiable]: associations/trait.Identifiable.html
///
/// # Options
///
/// ## foreign_key
///
/// Required. The name of the foreign key column for this association.
///
/// # Examples
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # table! { users { id -> Integer, } }
/// # table! { posts { id -> Integer, user_id -> Integer, } }
/// pub struct User {
///     id: i32,
/// }
/// # impl_Identifiable! { #[table_name(users)] struct User { id: i32, } }
///
/// pub struct Post {
///     id: i32,
///     user_id: i32,
/// }
/// # impl_Identifiable! { #[table_name(posts)] struct Post { id: i32, user_id: i32, } }
///
/// HasMany! {
///     (posts, foreign_key = user_id)
///     #[table_name(users)]
///     struct User {
///         id: i32,
///     }
/// }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! HasMany {
    // Format arguments
    (
        ($child_table_name:ident, foreign_key = $foreign_key_name:ident)
        $($rest:tt)*
    ) => {
        HasMany! {
            (
                child_table = $child_table_name::table,
                foreign_key = $child_table_name::$foreign_key_name,
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
        HasMany! {
            (
                parent_table_name = $table_name,
                $($args)*
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
        HasMany! {
            $args
            $($body)*
        }
    };

    // Receive parsed fields of normal struct from `__diesel_parse_struct_body`
    // These patterns must appear above those which start with an ident to compile
    (
        (
            parent_table_name = $parent_table_name:ident,
            child_table = $child_table:path,
            foreign_key = $foreign_key:path,
        ),
        fields = [$({
            field_name: $field_name:ident,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
            $($rest:tt)*
        })+],
    ) => {
        joinable_inner! {
            left_table_ty = $parent_table_name::table,
            right_table_ty = $child_table,
            right_table_expr = $child_table,
            foreign_key = $foreign_key,
            primary_key_ty = <$parent_table_name::table as $crate::query_source::Table>::PrimaryKey,
            primary_key_expr = $crate::Table::primary_key(&$parent_table_name::table),
        }
    };

    // Handle struct with no generics
    (
        $args:tt
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            $args,
            callback = HasMany,
            body = $body,
        }
    };
}
