#[macro_export]
#[doc(hidden)]
macro_rules! impl_Identifiable {
    // Find table name in options
    (
        $(())*
        $(#[$option_name:ident $option_value:tt])*
        $(pub)* struct $($rest:tt)*
    ) => {
        __diesel_find_option_with_name! {
            (options = [$(#[$option_name $option_value])*], body = $($rest)*),
            option_name = table_name,
            args = [$(#[$option_name $option_value])*],
            callback = impl_Identifiable,
        }
    };

    // Next, find primary key name in options
    (
        (options = $options:tt, $($args:tt)*),
        found_option_with_name = table_name,
        value = ($table_name:ident),
    ) => {
        __diesel_find_option_with_name! {
            (options = $options, table_name = $table_name, $($args)*),
            option_name = primary_key,
            args = $options,
            callback = impl_Identifiable,
            default = (id),
        }
    };

    // Options extracted, deal with the struct body (bottom two branches)
    (
        (
            options = $ignore:tt,
            table_name = $table_name:ident,
            body = $($body:tt)*
        ),
        found_option_with_name = primary_key,
        value = $primary_key_names:tt,
    ) => {
        impl_Identifiable! {
            (
                table_name = $table_name,
                primary_key_names = $primary_key_names,
            )
            $($body)*
        }
    };

    // We found the primary key field, return the final impl
    (
        (
            table_name = $table_name:ident,
            struct_ty = $struct_ty:ty,
            lifetimes = ($($lifetimes:tt),*),
        ),
        found_fields_with_field_names,
        fields = [$({
            field_name: $field_name:ident,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
            $($rest:tt)*
        })*],
    ) => {
        impl<$($lifetimes),*> $crate::associations::HasTable for $struct_ty {
            type Table = $table_name::table;

            fn table() -> Self::Table {
                $table_name::table
            }
        }

        impl<'ident $(,$lifetimes)*> $crate::associations::Identifiable for &'ident $struct_ty {
            type Id = ($(&'ident $field_ty),*);

            fn id(self) -> Self::Id {
                ($(&self.$field_name),*)
            }
        }
    };

    // Search for the primary key fields and continue
    (
        (
            table_name = $table_name:ident,
            primary_key_names = $primary_key_names:tt,
            $($args:tt)*
        ),
        fields = $fields:tt,
    ) => {
        __diesel_fields_with_field_names! {
            (table_name = $table_name, $($args)*),
            callback = impl_Identifiable,
            targets = $primary_key_names,
            fields = $fields,
        }
    };

    // Handle struct with generic lifetimes
    (
        ($($args:tt)*)
        $struct_name:ident <$($lifetimes:tt),*>
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                $($args)*
                struct_ty = $struct_name<$($lifetimes),*>,
                lifetimes = ($($lifetimes),*),
            ),
            callback = impl_Identifiable,
            body = $body,
        }
    };

    // Handle struct with no generics
    (
        ($($args:tt)*)
        $struct_name:ident
        $body:tt $(;)*
    ) => {
        __diesel_parse_struct_body! {
            (
                $($args)*
                struct_ty = $struct_name,
                lifetimes = (),
            ),
            callback = impl_Identifiable,
            body = $body,
        }
    };
}

#[cfg(test)]
mod test {
    table! {
        foos {
            id -> Integer,
        }
    }

    table! {
        bars {
            id -> VarChar,
        }
    }

    #[test]
    fn derive_identifiable_on_simple_struct() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations)]
        struct Foo {
            id: i32,
            #[allow(dead_code)] foo: i32,
        }

        impl_Identifiable! {
            #[table_name(foos)]
            struct Foo {
                id: i32,
                foo: i32,
            }
        }

        let foo1 = Foo { id: 1, foo: 2 };
        let foo2 = Foo { id: 2, foo: 3 };
        assert_eq!(&1, foo1.id());
        assert_eq!(&2, foo2.id());
    }

    #[test]
    fn derive_identifiable_when_id_is_not_first_field() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations)]
        struct Foo {
            #[allow(dead_code)] foo: i32,
            id: i32,
        }

        impl_Identifiable! {
            #[table_name(foos)]
            struct Foo {
                foo: i32,
                id: i32,
            }
        }

        let foo1 = Foo { id: 1, foo: 2 };
        let foo2 = Foo { id: 2, foo: 3 };
        assert_eq!(&1, foo1.id());
        assert_eq!(&2, foo2.id());
    }

    #[test]
    fn derive_identifiable_on_struct_with_non_integer_pk() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations)]
        struct Foo {
            id: &'static str,
            #[allow(dead_code)] foo: i32,
        }

        impl_Identifiable! {
            #[table_name(bars)]
            struct Foo {
                id: &'static str,
                foo: i32,
            }
        }

        let foo1 = Foo { id: "hi", foo: 2 };
        let foo2 = Foo {
            id: "there",
            foo: 3,
        };
        assert_eq!(&"hi", foo1.id());
        assert_eq!(&"there", foo2.id());
    }

    #[test]
    fn derive_identifiable_on_struct_with_lifetime() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations)]
        struct Foo<'a> {
            id: &'a str,
            #[allow(dead_code)] foo: i32,
        }

        impl_Identifiable! {
            #[table_name(bars)]
            struct Foo<'a> {
                id: &'a str,
                foo: i32,
            }
        }

        let foo1 = Foo { id: "hi", foo: 2 };
        let foo2 = Foo {
            id: "there",
            foo: 3,
        };
        assert_eq!(&"hi", foo1.id());
        assert_eq!(&"there", foo2.id());
    }

    #[test]
    fn derive_identifiable_with_non_standard_pk() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations, dead_code)]
        struct Foo<'a> {
            id: i32,
            foo_id: &'a str,
            foo: i32,
        }

        impl_Identifiable! {
            #[table_name(bars)]
            #[primary_key(foo_id)]
            struct Foo<'a> {
                id: i32,
                foo_id: &'a str,
                foo: i32,
            }
        }

        let foo1 = Foo {
            id: 1,
            foo_id: "hi",
            foo: 2,
        };
        let foo2 = Foo {
            id: 2,
            foo_id: "there",
            foo: 3,
        };
        assert_eq!(&"hi", foo1.id());
        assert_eq!(&"there", foo2.id());
    }

    #[test]
    fn derive_identifiable_with_non_standard_pk_given_before_table_name() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations, dead_code)]
        struct Foo<'a> {
            id: i32,
            foo_id: &'a str,
            foo: i32,
        }

        impl_Identifiable! {
            #[primary_key(foo_id)]
            #[table_name(bars)]
            struct Foo<'a> {
                id: i32,
                foo_id: &'a str,
                foo: i32,
            }
        }

        let foo1 = Foo {
            id: 1,
            foo_id: "hi",
            foo: 2,
        };
        let foo2 = Foo {
            id: 2,
            foo_id: "there",
            foo: 3,
        };
        assert_eq!(&"hi", foo1.id());
        assert_eq!(&"there", foo2.id());
    }

    #[test]
    fn derive_identifiable_with_composite_pk() {
        use associations::Identifiable;

        #[allow(missing_debug_implementations, missing_copy_implementations, dead_code)]
        struct Foo {
            id: i32,
            foo_id: i32,
            bar_id: i32,
            foo: i32,
        }

        impl_Identifiable! {
            #[primary_key(foo_id, bar_id)]
            #[table_name(bars)]
            struct Foo {
                id: i32,
                foo_id: i32,
                bar_id: i32,
                foo: i32,
            }
        }

        let foo1 = Foo {
            id: 1,
            foo_id: 2,
            bar_id: 3,
            foo: 4,
        };
        let foo2 = Foo {
            id: 5,
            foo_id: 6,
            bar_id: 7,
            foo: 8,
        };
        assert_eq!((&2, &3), foo1.id());
        assert_eq!((&6, &7), foo2.id());
    }
}
