/// Implements the [`Identifiable`][identifiable] trait for a given struct. This
/// macro should be called by copy/pasting the definition of the struct into it.
///
/// The struct must have a field called `id`, and the type of that field must be
/// `Copy`. This macro does not work with tuple structs.
///
/// [identifiable]: /diesel/associations/trait.Identifiable.html
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # table! { users { id -> Integer, } }
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// Identifiable! {
///     #[table_name(users)]
///     struct User {
///         id: i32,
///         name: String,
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
///     #[derive(Identifiable)]
///     #[table_name(users)]
///     struct User {
///         id: i32,
///         name: String,
///     }
/// }
/// ```
#[macro_export]
macro_rules! Identifiable {
    // Extract table name from meta item
    (
        $(())*
        #[table_name($table_name:ident)]
        $($rest:tt)*
    ) => {
        Identifiable! {
            (table_name = $table_name,)
            $($rest)*
        }
    };

    // Strip meta items that aren't table name
    (
        $args:tt
        #[$ignore:meta]
        $($rest:tt)*
    ) => {
        Identifiable!($args $($rest)*);
    };

    // Strip pub (if present) and struct from definition
    // After this step, we will go to the step at the bottom.
    (
        $args:tt
        $(pub)* struct $($body:tt)*
    ) => {
        Identifiable!($args $($body)*);
    };

    // We found the `id` field, return the final impl
    (
        (
            table_name = $table_name:ident,
            struct_ty = $struct_ty:ty,
        ),
        fields = [{
            field_name: id,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
        } $($fields:tt)*],
    ) => {
        impl $crate::associations::Identifiable for $struct_ty {
            type Id = $field_ty;
            type Table = $table_name::table;

            fn table() -> Self::Table {
                $table_name::table
            }

            fn id(&self) -> &Self::Id {
                &self.id
            }
        }
    };

    // Search for the `id` field and continue
    (
        $args:tt,
        fields = [{
            field_name: $field_name:ident,
            column_name: $column_name:ident,
            field_ty: $field_ty:ty,
            field_kind: $field_kind:ident,
        } $($fields:tt)*],
    ) => {
        Identifiable! {
            $args,
            fields = [$($fields)*],
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
            ),
            callback = Identifiable,
            body = $body,
        }
    };
}

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

    struct Foo {
        id: i32,
        #[allow(dead_code)]
        foo: i32,
    }

    Identifiable! {
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

    struct Foo {
        #[allow(dead_code)]
        foo: i32,
        id: i32,
    }

    Identifiable! {
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

    struct Foo {
        id: &'static str,
        #[allow(dead_code)]
        foo: i32,
    }

    Identifiable! {
        #[table_name(bars)]
        struct Foo {
            id: &'static str,
            foo: i32,
        }
    }

    let foo1 = Foo { id: "hi", foo: 2 };
    let foo2 = Foo { id: "there", foo: 3 };
    assert_eq!(&"hi", foo1.id());
    assert_eq!(&"there", foo2.id());
}
