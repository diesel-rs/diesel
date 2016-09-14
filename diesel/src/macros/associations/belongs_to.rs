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
    // Next we need to get the foreign key field in order to determine if it's optional.
    // These patterns must appear above those which start with an ident to compile
    (
        (
            struct_name = $struct_name:ident,
            parent_struct = $parent_struct:ident,
            foreign_key_name = $foreign_key_name:ident,
            $($remaining_arguments:tt)*
        ),
        fields = $fields:tt,
    ) => {
        __diesel_field_with_column_name! {
            (
                fields = $fields,
                struct_name = $struct_name,
                parent_struct = $parent_struct,
                foreign_key_name = $foreign_key_name,
                $($remaining_arguments)*
            ),
            callback = BelongsTo,
            target = $foreign_key_name,
            fields = $fields,
        }
    };

    // Receive the foreign key field from __diesel_field_with_column_name!
    (
        (
            fields = $fields:tt,
            $($remaining_args:tt)*
        ),
        found_field_with_column_name = $ignore:ident,
        field = {
            field_name: $ignore2:ident,
            column_name: $ignore3:ident,
            field_ty: $ignore4:ty,
            field_kind: $foreign_key_kind:ident,
        },
    ) => {
        BelongsTo! {
            (
                foreign_key_kind = $foreign_key_kind,
                $($remaining_args)*
            ),
            fields = $fields,
        }
    };

    // Generate code when FK is not optional
    (
        (
            foreign_key_kind = regular,
            struct_name = $struct_name:ident,
            parent_struct = $parent_struct:ident,
            foreign_key_name = $foreign_key_name:ident,
            child_table_name = $child_table_name:ident,
        ),
        $($rest:tt)*
    ) => {
        impl $crate::associations::BelongsTo<$parent_struct> for $struct_name {
            type ForeignKeyColumn = $child_table_name::$foreign_key_name;

            fn foreign_key(&self) -> Option<&<$parent_struct as $crate::associations::Identifiable>::Id> {
                Some(&self.$foreign_key_name)
            }

            fn foreign_key_column() -> Self::ForeignKeyColumn {
                $child_table_name::$foreign_key_name
            }
        }

        BelongsTo! {
            @generate_joins,
            (
                struct_name = $struct_name,
                parent_struct = $parent_struct,
                foreign_key_name = $foreign_key_name,
                child_table_name = $child_table_name,
            ),
            $($rest)*
        }
    };

    // Generate code when FK is optional
    (
        (
            foreign_key_kind = option,
            struct_name = $struct_name:ident,
            parent_struct = $parent_struct:ident,
            foreign_key_name = $foreign_key_name:ident,
            child_table_name = $child_table_name:ident,
        ),
        $($rest:tt)*
    ) => {
        impl $crate::associations::BelongsTo<$parent_struct> for $struct_name {
            type ForeignKeyColumn = $child_table_name::$foreign_key_name;

            fn foreign_key(&self) -> Option<&<$parent_struct as $crate::associations::Identifiable>::Id> {
                self.$foreign_key_name.as_ref()
            }

            fn foreign_key_column() -> Self::ForeignKeyColumn {
                $child_table_name::$foreign_key_name
            }
        }

        BelongsTo! {
            @generate_joins,
            (
                struct_name = $struct_name,
                parent_struct = $parent_struct,
                foreign_key_name = $foreign_key_name,
                child_table_name = $child_table_name,
            ),
            $($rest)*
        }
    };

    // Generate code that does not differ based on the fk being optional
    (
        @generate_joins,
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
