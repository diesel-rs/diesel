/// Implements the [`Insertable`][insertable] trait for a given struct. This
/// macro should be called with the name of the table you wish to use the struct
/// with, followed by the entire struct body.
///
/// [insertable]: prelude/trait.Insertable.html
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # table! { users { id -> Integer, name -> VarChar, hair_color -> Nullable<VarChar>, } }
/// struct NewUser<'a> {
///     name: &'a str,
///     hair_color: &'a str,
/// }
///
/// impl_Insertable! {
///     (users)
///     struct NewUser<'a> {
///         name: &'a str,
///         hair_color: &'a str,
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
///     #[derive(Insertable(users))]
///     struct NewUser<'a> {
///         name: &'a str,
///         hair_color: &'a str,
///     }
/// }
/// ```
///
/// You can also use this macro with tuple structs, but *all* fields must be
/// annotated with `#[column_name(name)]`. Additionally, a trailing comma after
/// the last field is required.
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # table! { users { id -> Integer, name -> VarChar, hair_color -> Nullable<VarChar>, } }
/// struct NewUser<'a>(&'a str, Option<&'a str>);
///
/// impl_Insertable! {
///     (users)
///     struct NewUser<'a>(
///         #[column_name(name)]
///         &'a str,
///         #[column_name(hair_color)]
///         Option<&'a str>,
///     );
/// }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! impl_Insertable {
    // Strip meta items, pub (if present) and struct from definition
    (
        ($table_name:ident)
        $(#[$ignore:meta])*
        $(pub)* struct $($body:tt)*
    ) => {
        impl_Insertable! {
            ($table_name)
            $($body)*
        }
    };

    // Handle struct with lifetimes
    (
        ($table_name:ident)
        $struct_name:ident <$($lifetime:tt),*>
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                table_name = $table_name,
                struct_ty = $struct_name<$($lifetime),*>,
                lifetimes = ($($lifetime),*),
            ),
            callback = impl_Insertable,
            body = $body,
        }
    };

    // Handle struct with no lifetimes
    (
        ($table_name:ident)
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                table_name = $table_name,
                struct_ty = $struct_name,
                lifetimes = (),
            ),
            callback = impl_Insertable,
            body = $body,
        }
    };

    // Receive parsed fields of tuple struct from `__diesel_parse_struct_body`
    (
        (
            struct_name = $struct_name:ident,
            $($headers:tt)*
        ),
        fields = [$({
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
            $($rest:tt)*
        })+],
    ) => {
        impl_Insertable! {
            $($headers)*
            self_to_columns = $struct_name($(ref $column_name),+),
            columns = ($($column_name, $field_ty, $field_kind),+),
        }
    };

    // Receive parsed fields of normal struct from `__diesel_parse_struct_body`
    (
        (
            struct_name = $struct_name:ident,
            $($headers:tt)*
        ),
        fields = [$({
            field_name: $field_name:ident,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
            $($rest:tt)*
        })+],
    ) => {
        impl_Insertable! {
            $($headers)*
            self_to_columns = $struct_name { $($field_name: ref $column_name),+ },
            columns = ($($column_name, $field_ty, $field_kind),+),
        }
    };

    (
        table_name = $table_name:ident,
        struct_ty = $struct_ty:ty,
        lifetimes = ($($lifetime:tt),*),
        self_to_columns = $self_to_columns:pat,
        columns = ($($column_name:ident, $field_ty:ty, $field_kind:ident),+),
    ) => { __diesel_parse_as_item! {
        impl<$($lifetime,)* 'insert, DB> $crate::insertable::Insertable<$table_name::table, DB>
            for &'insert $struct_ty where
                DB: $crate::backend::Backend,
                Hlist!($(
                    $crate::insertable::ColumnInsertValue<
                        $table_name::$column_name,
                        $crate::expression::helper_types::AsNullableExpr<
                            &'insert $field_ty,
                            $table_name::$column_name,
                        >,
                    >
                ,)+): $crate::insertable::InsertValues<DB>,
        {
            type Values = Hlist!($(
                $crate::insertable::ColumnInsertValue<
                    $table_name::$column_name,
                    $crate::expression::helper_types::AsNullableExpr<
                        &'insert $field_ty,
                        $table_name::$column_name,
                    >,
                >
            ,)+);

            #[allow(non_shorthand_field_patterns)]
            fn values(self) -> Self::Values {
                use $crate::expression::{AsExpression, Expression};
                use $crate::insertable::ColumnInsertValue;
                use $crate::types::IntoNullable;
                let $self_to_columns = *self;
                hlist!($(
                    Insertable_column_expr!($table_name::$column_name, $column_name, $field_kind)
                ,)+)
            }
        }

    } __diesel_parse_as_item! {
        impl<$($lifetime: 'insert,)* 'insert, Op> $crate::query_builder::insert_statement::IntoInsertStatement<$table_name::table, Op>
            for &'insert $struct_ty
        {
            type InsertStatement = $crate::query_builder::insert_statement::InsertStatement<$table_name::table, Self, Op>;

            fn into_insert_statement(self, target: $table_name::table, operator: Op) -> Self::InsertStatement {
                $crate::query_builder::insert_statement::InsertStatement::no_returning_clause(
                    target,
                    self,
                    operator,
                )
            }
        }
    } __diesel_parse_as_item! {
        impl<$($lifetime: 'insert,)* 'insert> $crate::query_builder::insert_statement::UndecoratedInsertRecord<$table_name::table>
            for &'insert $struct_ty
        {
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! Insertable_column_expr {
    ($column:path, $field_access:expr, option) => {
        match $field_access {
            value @ &Some(_) => Insertable_column_expr!($column, value, regular),
            &None => ColumnInsertValue::Default($column),
        }
    };

    ($column:path, $field_access:expr, regular) => {
        ColumnInsertValue::Expression(
            $column,
            AsExpression::<<<$column as Expression>::SqlType as IntoNullable>::Nullable>
                ::as_expression($field_access),
        )
    };
}

#[cfg(test)]
#[allow(missing_debug_implementations, missing_copy_implementations)]
mod tests {
    use prelude::*;

    table! {
        users {
            id -> Integer,
            name -> VarChar,
            hair_color -> Nullable<VarChar>,
        }
    }

    #[test]
    fn simple_struct_definition() {
        struct NewUser {
            name: String,
            hair_color: String,
        }

        impl_Insertable! {
            (users)
            struct NewUser {
                name: String,
                hair_color: String,
            }
        }

        let conn = connection();
        let new_user = NewUser { name: "Sean".into(), hair_color: "Black".into() };
        ::insert(&new_user).into(users::table).execute(&conn).unwrap();

        let saved = users::table.select(hlist!(users::name, users::hair_color))
            .load::<(String, Option<String>)>(&conn);
        let expected = vec![("Sean".to_string(), Some("Black".to_string()))];
        assert_eq!(Ok(expected), saved);
    }

    macro_rules! test_struct_definition {
        ($test_name:ident, $($struct_def:tt)*) => {
            // FIXME: This module is to work around rust-lang/rust#31776
            // Remove the module and move the struct definition into the test function once
            // 1.9 is released. The `use` statements can be removed.
            //
            // The indentation is intentionally weird to avoid git churn when this is fixed.
            mod $test_name {
                use super::{users, connection};
                use prelude::*;
                __diesel_parse_as_item!($($struct_def)*);
            #[test]
            fn $test_name() {
                impl_Insertable! {
                    (users)
                    $($struct_def)*
                }

                let conn = connection();
                let new_user = NewUser { name: "Sean".into(), hair_color: None };
                ::insert(&new_user).into(users::table).execute(&conn).unwrap();

                let saved = users::table.select(hlist!(users::name, users::hair_color))
                    .load::<(String, Option<String>)>(&conn);
                let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
                assert_eq!(Ok(expected), saved);
            }
            }
        }
    }

    test_struct_definition! {
        struct_with_option_field,
        struct NewUser {
            name: String,
            hair_color: Option<String>,
        }
    }

    test_struct_definition! {
        pub_struct_definition,
        pub struct NewUser {
            name: String,
            hair_color: Option<String>,
        }
    }

    test_struct_definition! {
        struct_with_pub_field,
        pub struct NewUser {
            pub name: String,
            hair_color: Option<String>,
        }
    }

    test_struct_definition! {
        struct_with_pub_option_field,
        pub struct NewUser {
            name: String,
            pub hair_color: Option<String>,
        }
    }

    test_struct_definition! {
        named_struct_with_borrowed_body,
        struct NewUser<'a> {
            name: &'a str,
            hair_color: Option<&'a str>,
        }
    }

    test_struct_definition! {
        named_struct_without_trailing_comma,
        struct NewUser<'a> {
            name: &'a str,
            hair_color: Option<&'a str>
        }
    }

    #[test]
    fn named_struct_with_renamed_field() {
        struct NewUser {
            my_name: String,
            hair_color: String,
        }

        impl_Insertable! {
            (users)
            struct NewUser {
                #[column_name(name)]
                my_name: String,
                hair_color: String,
            }
        }

        let conn = connection();
        let new_user = NewUser { my_name: "Sean".into(), hair_color: "Black".into() };
        ::insert(&new_user).into(users::table).execute(&conn).unwrap();

        let saved = users::table.select(hlist!(users::name, users::hair_color))
            .load::<(String, Option<String>)>(&conn);
        let expected = vec![("Sean".to_string(), Some("Black".to_string()))];
        assert_eq!(Ok(expected), saved);
    }

    #[test]
    fn named_struct_with_renamed_option_field() {
        struct NewUser {
            my_name: String,
            my_hair_color: Option<String>,
        }

        impl_Insertable! {
            (users)
            struct NewUser {
                #[column_name(name)]
                my_name: String,
                #[column_name(hair_color)]
                my_hair_color: Option<String>,
            }
        }

        let conn = connection();
        let new_user = NewUser { my_name: "Sean".into(), my_hair_color: None };
        ::insert(&new_user).into(users::table).execute(&conn).unwrap();

        let saved = users::table.select(hlist!(users::name, users::hair_color))
            .load::<(String, Option<String>)>(&conn);
        let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
        assert_eq!(Ok(expected), saved);
    }

    #[test]
    fn tuple_struct() {
        struct NewUser<'a>(
            &'a str,
            Option<&'a str>,
        );

        impl_Insertable! {
            (users)
            struct NewUser<'a>(
                #[column_name(name)]
                pub &'a str,
                #[column_name(hair_color)]
                Option<&'a str>,
            );
        }

        let conn = connection();
        let new_user = NewUser("Sean", None);
        ::insert(&new_user).into(users::table).execute(&conn).unwrap();

        let saved = users::table.select(hlist!(users::name, users::hair_color))
            .load::<(String, Option<String>)>(&conn);
        let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
        assert_eq!(Ok(expected), saved);
    }

    #[test]
    fn tuple_struct_without_trailing_comma() {
        struct NewUser<'a>(
            &'a str,
            Option<&'a str>
        );

        impl_Insertable! {
            (users)
            struct NewUser<'a>(
                #[column_name(name)]
                pub &'a str,
                #[column_name(hair_color)]
                Option<&'a str>
            );
        }

        let conn = connection();
        let new_user = NewUser("Sean", None);
        ::insert(&new_user).into(users::table).execute(&conn).unwrap();

        let saved = users::table.select(hlist!(users::name, users::hair_color))
            .load::<(String, Option<String>)>(&conn);
        let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
        assert_eq!(Ok(expected), saved);
    }

    cfg_if! {
        if #[cfg(feature = "sqlite")] {
            fn connection() -> ::test_helpers::TestConnection {
                let conn = ::test_helpers::connection();
                conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR NOT NULL, hair_color VARCHAR DEFAULT 'Green')").unwrap();
                conn
            }
        } else if #[cfg(feature = "postgres")] {
            fn connection() -> ::test_helpers::TestConnection {
                let conn = ::test_helpers::connection();
                conn.execute("DROP TABLE IF EXISTS users").unwrap();
                conn.execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR NOT NULL, hair_color VARCHAR DEFAULT 'Green')").unwrap();
                conn
            }

            // FIXME: This can be moved into the function once `pub` is allowed
            table! {
                posts {
                    id -> Serial,
                    tags -> Array<Text>,
                }
            }

            #[test]
            fn insertable_with_slice_of_borrowed() {
                struct NewPost<'a> { tags: &'a [&'a str], }
                impl_Insertable! { (posts) struct NewPost<'a> { tags: &'a [&'a str], } }

                let conn = ::test_helpers::connection();
                conn.execute("DROP TABLE IF EXISTS posts").unwrap();
                conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
                let new_post = NewPost { tags: &["hi", "there"] };
                ::insert(&new_post).into(posts::table).execute(&conn).unwrap();

                let saved = posts::table.select(posts::tags).load::<Vec<String>>(&conn);
                let expected = vec![vec![String::from("hi"), String::from("there")]];
                assert_eq!(Ok(expected), saved);
            }
        } else if #[cfg(feature = "mysql")] {
            fn connection() -> ::test_helpers::TestConnection {
                let conn = ::test_helpers::connection_no_transaction();
                conn.execute("DROP TABLE IF EXISTS users").unwrap();
                conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTO_INCREMENT, name TEXT NOT NULL, hair_color VARCHAR(255) DEFAULT 'Green')").unwrap();
                conn.begin_test_transaction().unwrap();
                conn
            }
        } else {
            // FIXME: https://github.com/rust-lang/rfcs/pull/1695
            // compile_error!("At least one backend must be enabled to run tests");
        }
    }
}
