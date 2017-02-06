/// Implements the [`Queryable`][queryable] trait for a given struct. This macro
/// should be called by copy/pasting the definition of the struct into it.
///
/// [queryable]: query_source/trait.Queryable.html
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// struct User {
///     name: String,
///     hair_color: Option<String>,
/// }
///
/// impl_Queryable! {
///     struct User {
///         name: String,
///         hair_color: Option<String>,
///     }
/// }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! impl_Queryable {
    // Strip empty argument list if given (Passed by custom_derive macro)
    (() $($body:tt)*) => {
        impl_Queryable! {
            $($body)*
        }
    };

    // Strip meta items, pub (if present) and struct from definition
    (
        $(#[$ignore:meta])*
        $(pub)* struct $($body:tt)*
    ) => {
        impl_Queryable! {
            $($body)*
        }
    };

    // Receive parsed fields of normal struct from `__diesel_parse_struct_body`
    // These patterns must appear above those which start with an ident to compile
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
        impl_Queryable! {
            $($headers)*
            row_ty = ($($field_ty,)+),
            row_pat = ($($field_name,)+),
            build_expr = $struct_name { $($field_name: $field_name),+ },
        }
    };

    // Receive parsed fields of tuple struct from `__diesel_parse_struct_body`
    // where the fields were annotated with `#[column_name]`. We don't need the
    // name, so toss it out.
    (
        $headers:tt,
        fields = [$({
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
            $($rest:tt)*
        })+],
    ) => {
        impl_Queryable! {
            $headers,
            fields = [$({
                field_ty: $field_ty,
                field_kind: $field_kind,
                $($rest:tt)*
            })+],
        }
    };

    // Receive parsed fields of tuple struct from `__diesel_parse_struct_body`
    (
        (
            struct_name = $struct_name:ident,
            $($headers:tt)*
        ),
        fields = [$({
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
            $($rest:tt)*
        })+],
    ) => {
        impl_Queryable! {
            $($headers)*
            row_ty = ($($field_ty,)+),
            row_pat = ($($field_kind,)+),
            build_expr = $struct_name($($field_kind),+),
        }
    };

    // Construct the final impl
    (
        struct_ty = $struct_ty:ty,
        generics = ($($generics:ident),*),
        lifetimes = ($($lifetimes:tt),*),
        row_ty = $row_ty:ty,
        row_pat = $row_pat:pat,
        build_expr = $build_expr:expr,
    ) => {
        impl<$($lifetimes,)* $($generics,)* __DB, __ST> $crate::Queryable<__ST, __DB> for $struct_ty where
            __DB: $crate::backend::Backend + $crate::types::HasSqlType<__ST>,
            $row_ty: $crate::types::FromSqlRow<__ST, __DB>,
        {
            type Row = $row_ty;

            fn build(row: Self::Row) -> Self {
                let $row_pat = row;
                $build_expr
            }
        }
    };

    // Handle struct with generics
    (
        $struct_name:ident <$($generics:ident),*>
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                struct_ty = $struct_name<$($generics),*>,
                generics = ($($generics),*),
                lifetimes = (),
            ),
            callback = impl_Queryable,
            body = $body,
        }
    };

    // Handle struct with no generics
    (
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                struct_name = $struct_name,
                struct_ty = $struct_name,
                generics = (),
                lifetimes = (),
            ),
            callback = impl_Queryable,
            body = $body,
        }
    };
}

#[cfg(test)]
mod tests {
    use expression::dsl::sql;
    use prelude::*;
    use test_helpers::connection;

    cfg_if! {
        if #[cfg(feature = "mysql")] {
            type IntLiteralSql = ::types::BigInt;
            type IntLiteralRust = i64;
        } else {
            type IntLiteralSql = ::types::Integer;
            type IntLiteralRust = i32;
        }
    }

    #[test]
    fn named_struct_definition() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct MyStruct {
            foo: IntLiteralRust,
            bar: IntLiteralRust,
        }

        impl_Queryable! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            struct MyStruct {
                foo: IntLiteralRust,
                bar: IntLiteralRust,
            }
        }

        let conn = connection();
        let data = ::select(sql::<(IntLiteralSql, IntLiteralSql)>("1, 2")).get_result(&conn);
        assert_eq!(Ok(MyStruct { foo: 1, bar: 2 }), data);
    }

    #[test]
    fn tuple_struct() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct MyStruct(IntLiteralRust, IntLiteralRust);

        impl_Queryable! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            struct MyStruct(#[column_name(foo)] IntLiteralRust, #[column_name(bar)] IntLiteralRust);
        }

        let conn = connection();
        let data = ::select(sql::<(IntLiteralSql, IntLiteralSql)>("1, 2")).get_result(&conn);
        assert_eq!(Ok(MyStruct(1, 2)), data);
    }

    #[test]
    fn tuple_struct_without_column_name_annotations() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct MyStruct(IntLiteralRust, IntLiteralRust);

        impl_Queryable! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            struct MyStruct(IntLiteralRust, IntLiteralRust);
        }

        let conn = connection();
        let data = ::select(sql::<(IntLiteralSql, IntLiteralSql)>("1, 2")).get_result(&conn);
        assert_eq!(Ok(MyStruct(1, 2)), data);
    }
}
