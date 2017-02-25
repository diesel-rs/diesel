/// Implements the [`AsChangeset`][changeset] trait for a given struct. This
/// macro should be called with the name of the table you wish to use the struct
/// with, followed by the entire struct body. This macro mirrors
/// `#[as_changeset]` from [`diesel_codegen`][diesel_codegen]
///
/// [changeset]: query_builder/trait.AsChangeset.html
/// [diesel_codegen]: https://github.com/diesel-rs/diesel/tree/master/diesel_codegen
///
/// # Options
///
/// - `treat_none_as_null` (boolean)
///     - Default value: `"false"`
///     - When set to `"true"`, option fields will set the column to `NULL` when their value is
///       `None`. When set to `"false"`, the field will not be assigned.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate diesel_codegen;
/// # #[macro_use] extern crate diesel;
/// # table! { users { id -> Integer, name -> VarChar, } }
/// # include!("src/doctest_setup.rs");
///
/// #[derive(PartialEq, Debug, Queryable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// impl_AsChangeset! {
///     (users)
///     struct User {
///         id: i32,
///         name: String,
///     }
/// }
///
/// #
/// # impl User {
/// #     fn new(id: i32, name: &str) -> Self {
/// #         User {
/// #             id: id,
/// #             name: name.into(),
/// #         }
/// #     }
/// # }
///
///
/// # fn main() {
/// #     use users::dsl::*;
/// #     let connection = establish_connection();
/// diesel::insert(&NewUser::new("Sean"))
///     .into(users)
///     .execute(&connection)
///     .unwrap();
/// let user_id = users.select(id).order(id.desc()).first(&connection).unwrap();
/// let changes = User::new(user_id, "Jim");
/// diesel::update(users.find(user_id))
///     .set(&changes)
///     .execute(&connection)
///     .unwrap();
///
/// let user_in_db = users.find(user_id).first(&connection);
/// assert_eq!(Ok(changes), user_in_db);
/// # }
/// ```
#[macro_export]
macro_rules! impl_AsChangeset {
    // Provide a default value for treat_none_as_null if not provided
    (
        ($table_name:ident)
        $($body:tt)*
    ) => {
        impl_AsChangeset! {
            ($table_name, treat_none_as_null="false")
            $($body)*
        }
    };

    // Strip meta items, pub (if present) and struct from definition
    (
        $args:tt
        $(#[$ignore:meta])*
        $(pub)* struct $($body:tt)*
    ) => {
        impl_AsChangeset! {
            $args
            $($body)*
        }
    };

    // Handle struct with lifetimes
    (
        ($table_name:ident, treat_none_as_null=$treat_none_as_null:tt)
        $struct_name:ident <$($lifetime:tt),*>
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                table_name = $table_name,
                treat_none_as_null = $treat_none_as_null,
                struct_ty = $struct_name<$($lifetime),*>,
                lifetimes = ($($lifetime),*),
            ),
            callback = impl_AsChangeset,
            body = $body,
        }
    };

    // Handle struct with no lifetimes. We pass a dummy lifetime to reduce
    // the amount of branching later.
    (
        ($table_name:ident, treat_none_as_null=$treat_none_as_null:tt)
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                table_name = $table_name,
                treat_none_as_null = $treat_none_as_null,
                struct_ty = $struct_name,
                lifetimes = ('a),
            ),
            callback = impl_AsChangeset,
            body = $body,
        }
    };

    // Receive parsed fields of struct from `__diesel_parse_struct_body`
    (
        (
            struct_name = $struct_name:ident,
            table_name = $table_name:ident,
            treat_none_as_null = $treat_none_as_null:tt,
            $($headers:tt)*
        ),
        fields = [$($field:tt)+],
    ) => {
        impl_AsChangeset! {
            (
                fields = [$($field)+],
                struct_name = $struct_name,
                table_name = $table_name,
                treat_none_as_null = $treat_none_as_null,
                $($headers)*
            ),
            changeset_ty = Hlist!($(
                AsChangeset_changeset_ty! {
                    table_name = $table_name,
                    treat_none_as_null = $treat_none_as_null,
                    field = $field,
                }
            ,)+),
        }
    };

    // Receive changeset ty when tuple struct from `AsChangeset_construct_changeset_ty`
    (
        (
            fields = [$({
                column_name: $column_name:ident,
                field_ty: $field_ty:ty,
                field_kind: $field_kind:ident,
                $($rest:tt)*
            })+],
            struct_name = $struct_name:ident,
            $($headers:tt)*
        ),
        changeset_ty = $changeset_ty:ty,
    ) => {
        impl_AsChangeset! {
            $($headers)*
            self_to_columns = $struct_name($(ref $column_name),+),
            columns = ($($column_name, $field_kind),+),
            field_names = [],
            changeset_ty = $changeset_ty,
        }
    };

    // Receive changeset ty when named struct from `AsChangeset_construct_changeset_ty`
    (
        (
            fields = [$({
                field_name: $field_name:ident,
                column_name: $column_name:ident,
                field_ty: $field_ty:ty,
                field_kind: $field_kind:ident,
                $($rest:tt)*
            })+],
            struct_name = $struct_name:ident,
            $($headers:tt)*
        ),
        changeset_ty = $changeset_ty:ty,
    ) => {
        impl_AsChangeset! {
            $($headers)*
            self_to_columns = $struct_name { $($field_name: ref $column_name,)+ ..},
            columns = ($($column_name, $field_kind),+),
            field_names = [$($field_name)+],
            changeset_ty = $changeset_ty,
        }
    };

    // Construct final impl
    (
        table_name = $table_name:ident,
        treat_none_as_null = $treat_none_as_null:tt,
        struct_ty = $struct_ty:ty,
        lifetimes = ($($lifetime:tt),*),
        self_to_columns = $self_to_columns:pat,
        columns = ($($column_name:ident, $field_kind:ident),+),
        field_names = $field_names:tt,
        changeset_ty = $changeset_ty:ty,
    ) => {
        __diesel_parse_as_item! {
            impl<$($lifetime,)* 'update> $crate::query_builder::AsChangeset
                for &'update $struct_ty
            {
                type Target = $table_name::table;
                type Changeset = $changeset_ty;

                #[allow(non_shorthand_field_patterns)]
                fn as_changeset(self) -> Self::Changeset {
                    use $crate::prelude::ExpressionMethods;
                    let $self_to_columns = *self;
                    hlist!($(
                        AsChangeset_column_expr!(
                            $table_name::$column_name,
                            $column_name,
                            none_as_null = $treat_none_as_null,
                            field_kind = $field_kind,
                        )
                    ,)+)
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! AsChangeset_changeset_ty {
    // Handle option field when treat none as null is false
    (
        table_name = $table_name:ident,
        treat_none_as_null = "false",
        field = {
            $(field_name: $field_name:ident,)*
            column_name: $column_name:ident,
            field_ty: $ignore:ty,
            field_kind: option,
            inner_field_ty: $field_ty:ty,
            $($rest:tt)*
        },
    ) => {
        Option<$crate::expression::helper_types::Eq<
            $table_name::$column_name,
            &'update $field_ty,
        >>
    };

    // Handle normal field or option when treat none as null is true
    (
        table_name = $table_name:ident,
        treat_none_as_null = $treat_none_as_null:tt,
        field = {
            $(field_name: $field_name:ident,)*
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            $($ignore:tt)*
        },
    ) => {
        $crate::expression::helper_types::Eq<
            $table_name::$column_name,
            &'update $field_ty,
        >
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! AsChangeset_column_expr {
    // When none_as_null is false, we don't update fields which aren't present
    (
        $column:expr,
        $field_access:expr,
        none_as_null = "false",
        field_kind = option,
    ) => {
        $field_access.as_ref().map(|f| $column.eq(f))
    };

    // If none_as_null is true, or the field kind isn't option, assign blindly
    (
        $column:expr,
        $field_access:expr,
        $($args:tt)*
    ) => {
        $column.eq($field_access)
    };
}

#[cfg(test)]
mod using_as_changeset_with_any_imports {
    table!(users {
        id -> Integer,
        name -> VarChar,
    });

    #[allow(missing_debug_implementations)]
    struct Changes {
        id: i32,
        name: String,
    }

    impl_AsChangeset! {
        (users)
        struct Changes {
            id: i32,
            name: String,
        }
    }
}
